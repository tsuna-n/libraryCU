//! Local recovery records are authenticated, bounded, and published before any
//! source mutation. They are not a transaction log for arbitrary commands.
use super::*;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_RECORD_BYTES: u64 = MAX_SOURCE_BYTES * 12 + 8192;
const MAX_RECOVERY_RECORDS: usize = 100;
const MAX_RECOVERY_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);
const RECOVERY_KEY_BYTES: usize = 32;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u32,
    root: PathBuf,
    path: String,
    original: String,
    applied: String,
    created_at_unix: u64,
    authentication: String,
}

#[derive(Serialize)]
struct AuthenticatedFields<'a> {
    version: u32,
    root: &'a Path,
    path: &'a str,
    original: &'a str,
    applied: &'a str,
    created_at_unix: u64,
}

pub fn prepare(target: &Target, patch: &Patch) -> Result<String> {
    target.check_unchanged()?;
    target.validate_response(&serde_json::to_string(&Replacement {
        before: patch.before.clone(),
        after: patch.after.clone(),
    })?)?;
    ensure!(patch.path == target.relative, "patch target mismatch");

    let lbc_directory = target.root.join(".lbc");
    let _lock = security::storage::lock_exclusive(&lbc_directory.join(".fixes.lock"))?;
    let directory = lbc_directory.join("fixes");
    security::files::reject_symlinks(&directory)?;
    fs::create_dir_all(&directory)?;
    security::files::reject_symlinks(&directory)?;

    let now = SystemTime::now();
    prune_records(&directory, now)?;
    let key = recovery_key(&lbc_directory)?;
    let mut record = Record {
        version: 2,
        root: target.root.clone(),
        path: target.relative.clone(),
        original: target.original.clone(),
        applied: target.original.replacen(&patch.before, &patch.after, 1),
        created_at_unix: now.duration_since(UNIX_EPOCH)?.as_secs(),
        authentication: String::new(),
    };
    record.authentication = authenticate(&record, &key)?;

    let mut temporary = tempfile::Builder::new()
        .prefix("fix-")
        .tempfile_in(&directory)?;
    let id = temporary
        .path()
        .file_name()
        .context("missing recovery ID")?
        .to_str()
        .context("invalid recovery ID")?
        .to_owned();
    let content = serde_json::to_vec(&record)?;
    ensure!(
        content.len() as u64 <= MAX_RECORD_BYTES,
        "recovery record is too large"
    );
    temporary.write_all(&content)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(directory.join(format!("{id}.json")))
        .context("cannot publish recovery record")?;
    security::storage::sync_directory(&directory)?;
    Ok(id)
}

pub fn restore(root: &Path, id: &str) -> Result<String> {
    ensure!(
        id.starts_with("fix-")
            && id.len() <= 100
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
        "invalid recovery ID"
    );
    security::files::reject_symlinks(root)?;
    let root = root.canonicalize()?;
    let lbc_directory = root.join(".lbc");
    let _lock = security::storage::lock_exclusive(&lbc_directory.join(".fixes.lock"))?;
    let content = security::files::read_text(
        &lbc_directory.join("fixes").join(format!("{id}.json")),
        MAX_RECORD_BYTES,
    )?;
    let record: Record = serde_json::from_str(&content).context(
        "invalid or legacy unauthenticated recovery record; inspect version 1 records manually",
    )?;
    ensure!(
        record.version == 2,
        "legacy unauthenticated recovery record; inspect it manually"
    );
    let key = read_recovery_key(&lbc_directory)?;
    verify_authentication(&record, &key)?;
    ensure!(
        record.root == root,
        "recovery record belongs to a different project"
    );
    let relative = Path::new(&record.path);
    ensure!(
        !relative.as_os_str().is_empty()
            && relative
                .components()
                .all(|c| matches!(c, Component::Normal(_))),
        "unsafe recovery target"
    );
    ensure!(
        !relative
            .components()
            .any(|c| crate::scanner::ignore::should_ignore_directory(
                Path::new(c.as_os_str()),
                true
            )),
        "hidden or generated recovery target"
    );
    ensure!(
        record.original.len() as u64 <= MAX_SOURCE_BYTES
            && record.applied.len() as u64 <= MAX_SOURCE_BYTES,
        "recovery source too large"
    );
    ensure!(
        security::redact_sensitive(&record.original) == record.original
            && security::redact_sensitive(&record.applied) == record.applied,
        "recovery source contains recognizable secrets"
    );
    let path = root.join(relative);
    let check = || -> Result<()> {
        let current = security::files::read_text(&path, MAX_SOURCE_BYTES)?;
        ensure!(
            path.canonicalize()?.starts_with(&root),
            "recovery target escaped project"
        );
        ensure!(
            current == record.applied,
            "rollback refused: source changed after application; preserve your edits and recover manually"
        );
        ensure!(
            !fs::metadata(&path)?.permissions().readonly(),
            "recovery target is read-only"
        );
        Ok(())
    };
    check()?;
    check()?;
    security::storage::atomic_replace(&path, record.original.as_bytes(), false)
        .context("cannot publish rollback atomically")?;
    Ok(record.path)
}

fn authenticate(record: &Record, key: &[u8]) -> Result<String> {
    let fields = AuthenticatedFields {
        version: record.version,
        root: &record.root,
        path: &record.path,
        original: &record.original,
        applied: &record.applied,
        created_at_unix: record.created_at_unix,
    };
    let mut mac = HmacSha256::new_from_slice(key).context("invalid recovery key")?;
    mac.update(&serde_json::to_vec(&fields)?);
    Ok(encode_hex(&mac.finalize().into_bytes()))
}

fn verify_authentication(record: &Record, key: &[u8]) -> Result<()> {
    let supplied = decode_hex(&record.authentication).context("invalid recovery authentication")?;
    let fields = AuthenticatedFields {
        version: record.version,
        root: &record.root,
        path: &record.path,
        original: &record.original,
        applied: &record.applied,
        created_at_unix: record.created_at_unix,
    };
    let mut mac = HmacSha256::new_from_slice(key).context("invalid recovery key")?;
    mac.update(&serde_json::to_vec(&fields)?);
    mac.verify_slice(&supplied)
        .map_err(|_| anyhow::anyhow!("recovery record authentication failed"))
}

fn recovery_key(directory: &Path) -> Result<Vec<u8>> {
    let path = directory.join("recovery.key");
    if path.exists() {
        return read_recovery_key(directory);
    }
    security::files::reject_symlinks(&path)?;
    let mut key = [0_u8; RECOVERY_KEY_BYTES];
    getrandom::fill(&mut key)
        .map_err(|error| anyhow::anyhow!("cannot generate recovery key: {error}"))?;
    let encoded = encode_hex(&key);
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    match options.open(&path) {
        Ok(mut file) => {
            file.write_all(encoded.as_bytes())?;
            file.sync_all()?;
            security::storage::sync_directory(directory)?;
            Ok(key.to_vec())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            read_recovery_key(directory)
        }
        Err(error) => Err(error).context("cannot create private recovery key"),
    }
}

fn read_recovery_key(directory: &Path) -> Result<Vec<u8>> {
    let path = directory.join("recovery.key");
    let encoded = security::files::read_text(&path, 256)?;
    let key = decode_hex(encoded.trim()).context("invalid recovery key")?;
    ensure!(
        key.len() == RECOVERY_KEY_BYTES,
        "invalid recovery key length"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let metadata = fs::metadata(&path)?;
        ensure!(
            metadata.uid() == unsafe { libc::geteuid() },
            "recovery key has a different owner"
        );
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "recovery key permissions must not grant group or other access"
        );
    }
    Ok(key)
}

fn prune_records(directory: &Path, now: SystemTime) -> Result<()> {
    let mut retained = Vec::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with("fix-") || !name.ends_with(".json") {
            continue;
        }
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }
        let modified = entry.metadata()?.modified()?;
        if recovery_record_expired(now, modified) {
            fs::remove_file(entry.path())?;
        } else {
            retained.push((modified, entry.path()));
        }
    }
    retained.sort();
    let remove_count = retained.len().saturating_sub(MAX_RECOVERY_RECORDS - 1);
    for (_, path) in retained.into_iter().take(remove_count) {
        fs::remove_file(path)?;
    }
    if remove_count > 0 {
        security::storage::sync_directory(directory)?;
    }
    Ok(())
}

fn recovery_record_expired(now: SystemTime, modified: SystemTime) -> bool {
    now.duration_since(modified)
        .is_ok_and(|age| age > MAX_RECOVERY_AGE)
}

fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(DIGITS[(byte >> 4) as usize] as char);
        encoded.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    ensure!(value.len().is_multiple_of(2), "hex value has odd length");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char)
                .to_digit(16)
                .context("invalid hex digit")?;
            let low = (pair[1] as char)
                .to_digit(16)
                .context("invalid hex digit")?;
            Ok(((high << 4) | low) as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recovery_fixture() -> (tempfile::TempDir, Target, Patch, String) {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("main.rs"), "old\n").unwrap();
        let diagnostic = Diagnostic {
            source: Some("rustc".into()),
            code: Some("E0308".into()),
            message: "mismatched types".into(),
            file: Some("main.rs".into()),
            line: Some(1),
            column: Some(1),
        };
        let target = Target::read(root.path(), &diagnostic).unwrap();
        let patch = target
            .validate_response(r#"{"before":"old","after":"new"}"#)
            .unwrap();
        let id = prepare(&target, &patch).unwrap();
        target.apply(&patch).unwrap();
        (root, target, patch, id)
    }

    #[test]
    fn pruning_is_bounded_and_does_not_follow_symlinks() {
        let root = tempfile::tempdir().unwrap();
        for index in 0..=MAX_RECOVERY_RECORDS {
            fs::write(root.path().join(format!("fix-{index:03}.json")), b"record").unwrap();
        }
        fs::write(root.path().join("keep.txt"), b"keep").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            root.path().join("keep.txt"),
            root.path().join("fix-link.json"),
        )
        .unwrap();

        prune_records(root.path(), SystemTime::now()).unwrap();

        let regular_records = fs::read_dir(root.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry.file_type().is_ok_and(|kind| kind.is_file())
                    && entry.file_name().to_string_lossy().starts_with("fix-")
            })
            .count();
        assert_eq!(regular_records, MAX_RECOVERY_RECORDS - 1);
        assert_eq!(fs::read(root.path().join("keep.txt")).unwrap(), b"keep");
        #[cfg(unix)]
        assert!(root.path().join("fix-link.json").is_symlink());
    }

    #[test]
    fn retention_age_expires_only_records_older_than_thirty_days() {
        let now = UNIX_EPOCH + Duration::from_secs(100 * 24 * 60 * 60);
        assert!(!recovery_record_expired(now, now - MAX_RECOVERY_AGE));
        assert!(recovery_record_expired(
            now,
            now - MAX_RECOVERY_AGE - Duration::from_secs(1)
        ));
        assert!(!recovery_record_expired(now, now + Duration::from_secs(1)));
    }

    #[test]
    fn missing_invalid_and_malformed_recovery_authentication_are_refused() {
        let (root, target, _patch, id) = recovery_fixture();
        let key_path = root.path().join(".lbc/recovery.key");
        let valid_key = fs::read(&key_path).unwrap();

        fs::remove_file(&key_path).unwrap();
        assert!(restore(root.path(), &id).is_err());
        assert_eq!(fs::read_to_string(&target.path).unwrap(), "new\n");

        fs::write(&key_path, "not-a-hex-key").unwrap();
        assert!(restore(root.path(), &id).is_err());
        assert_eq!(fs::read_to_string(&target.path).unwrap(), "new\n");

        fs::write(&key_path, valid_key).unwrap();
        let record_path = root.path().join(".lbc/fixes").join(format!("{id}.json"));
        fs::write(&record_path, "{malformed").unwrap();
        assert!(restore(root.path(), &id).is_err());
        assert_eq!(fs::read_to_string(&target.path).unwrap(), "new\n");
    }

    #[test]
    fn every_authenticated_recovery_field_is_tamper_evident() {
        let (root, target, _patch, id) = recovery_fixture();
        let record_path = root.path().join(".lbc/fixes").join(format!("{id}.json"));
        let original: serde_json::Value =
            serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();

        for (field, replacement) in [
            ("version", serde_json::json!(3)),
            ("root", serde_json::json!("/different-project")),
            ("path", serde_json::json!("other.rs")),
            ("original", serde_json::json!("different original\n")),
            ("applied", serde_json::json!("different applied\n")),
            ("created_at_unix", serde_json::json!(0)),
            ("authentication", serde_json::json!("00".repeat(32))),
        ] {
            let mut changed = original.clone();
            changed[field] = replacement;
            fs::write(&record_path, serde_json::to_vec(&changed).unwrap()).unwrap();
            assert!(
                restore(root.path(), &id).is_err(),
                "accepted tampered {field}"
            );
            assert_eq!(fs::read_to_string(&target.path).unwrap(), "new\n");
        }
    }

    #[test]
    fn legacy_unauthenticated_records_require_manual_recovery() {
        let (root, target, _patch, id) = recovery_fixture();
        let record_path = root.path().join(".lbc/fixes").join(format!("{id}.json"));
        fs::write(
            &record_path,
            serde_json::json!({
                "version": 1,
                "root": root.path(),
                "path": "main.rs",
                "original": "old\n",
                "applied": "new\n",
                "created_at_unix": 0
            })
            .to_string(),
        )
        .unwrap();

        let error = restore(root.path(), &id).unwrap_err().to_string();
        assert!(error.contains("legacy unauthenticated"), "{error}");
        assert_eq!(fs::read_to_string(&target.path).unwrap(), "new\n");
    }

    #[cfg(unix)]
    #[test]
    fn recovery_key_with_public_permissions_is_refused() {
        use std::os::unix::fs::PermissionsExt;

        let (root, target, _patch, id) = recovery_fixture();
        let key_path = root.path().join(".lbc/recovery.key");
        fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(restore(root.path(), &id).is_err());
        assert_eq!(fs::read_to_string(&target.path).unwrap(), "new\n");
    }
}
