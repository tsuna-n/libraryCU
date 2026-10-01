//! Private, authenticated proposals bind the reviewed replacement to one source snapshot.
use super::*;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_RECORD_BYTES: u64 = 64 * 1024;
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fields {
    version: u32,
    id: String,
    root: PathBuf,
    path: String,
    diagnostic_line: u32,
    original_sha256: String,
    before: String,
    after: String,
    created_at_unix: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    fields: Fields,
    authentication: String,
}

fn mac(fields: &Fields, key: &[u8]) -> Result<Hmac<Sha256>> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).context("invalid proposal key")?;
    mac.update(b"lbc-proposal-v1\0");
    mac.update(&serde_json::to_vec(fields)?);
    Ok(mac)
}

fn valid_id(id: &str) -> Result<()> {
    ensure!(
        id.starts_with("proposal-")
            && id.len() <= 100
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
        "invalid proposal ID"
    );
    Ok(())
}

pub fn save(target: &Target, patch: &Patch) -> Result<String> {
    target.check_unchanged()?;
    ensure!(patch.path == target.relative, "proposal target mismatch");
    target.validate_response(&serde_json::to_string(&Replacement {
        before: patch.before.clone(),
        after: patch.after.clone(),
    })?)?;
    let lbc = target.root.join(".lbc");
    // Share the recovery lock/key to avoid races between first-time key creation.
    let _lock = security::storage::lock_exclusive(&lbc.join(".fixes.lock"))?;
    let directory = lbc.join("proposals");
    security::files::reject_symlinks(&directory)?;
    security::permissions::validate_directory(&directory)?;
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    if !directory.exists() {
        builder.create(&directory)?;
    }
    security::permissions::validate_path(&directory, true)?;
    let now = SystemTime::now();
    let mut retained = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with("proposal-") || !name.ends_with(".json") {
            continue;
        }
        security::permissions::validate_path(&entry.path(), true)?;
        let modified = entry.metadata()?.modified()?;
        if now.duration_since(modified).is_ok_and(|age| age > MAX_AGE) {
            fs::remove_file(entry.path())?;
        } else {
            retained.push((modified, entry.path()));
        }
    }
    retained.sort();
    let remove = retained.len().saturating_sub(99);
    for (_, path) in retained.into_iter().take(remove) {
        fs::remove_file(path)?;
    }
    let key = rollback::recovery_key(&lbc)?;
    let mut temporary = tempfile::Builder::new()
        .prefix("proposal-")
        .tempfile_in(&directory)?;
    let id = temporary
        .path()
        .file_name()
        .context("missing proposal ID")?
        .to_str()
        .context("invalid proposal ID")?
        .to_owned();
    let fields = Fields {
        version: 1,
        id: id.clone(),
        root: target.root.clone(),
        path: target.relative.clone(),
        diagnostic_line: target.diagnostic_line,
        original_sha256: rollback::encode_hex(&Sha256::digest(target.original.as_bytes())),
        before: patch.before.clone(),
        after: patch.after.clone(),
        created_at_unix: now.duration_since(UNIX_EPOCH)?.as_secs(),
    };
    let authentication = rollback::encode_hex(&mac(&fields, &key)?.finalize().into_bytes());
    let bytes = serde_json::to_vec(&Record {
        fields,
        authentication,
    })?;
    ensure!(
        bytes.len() as u64 <= MAX_RECORD_BYTES,
        "proposal record too large"
    );
    security::permissions::validate_file(temporary.as_file(), true)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist_noclobber(directory.join(format!("{id}.json")))
        .context("cannot publish proposal")?;
    security::storage::sync_directory(&directory)?;
    Ok(id)
}

pub fn load(root: &Path, id: &str) -> Result<(Target, Patch)> {
    valid_id(id)?;
    security::files::reject_symlinks(root)?;
    let root = root.canonicalize()?;
    let lbc = root.join(".lbc");
    let _lock = security::storage::lock_exclusive(&lbc.join(".fixes.lock"))?;
    let directory = lbc.join("proposals");
    security::permissions::validate_path(&directory, true)?;
    let content = security::files::read_store_text(
        &directory.join(format!("{id}.json")),
        MAX_RECORD_BYTES,
        true,
    )?;
    let record: Record = serde_json::from_str(&content).context("invalid proposal record")?;
    let key = rollback::read_recovery_key(&lbc)?;
    mac(&record.fields, &key)?
        .verify_slice(&rollback::decode_hex(&record.authentication)?)
        .map_err(|_| anyhow::anyhow!("proposal authentication failed"))?;
    let fields = record.fields;
    ensure!(
        fields.version == 1 && fields.id == id,
        "proposal identity mismatch"
    );
    ensure!(
        fields.root == root,
        "proposal belongs to a different project"
    );
    let created = UNIX_EPOCH
        .checked_add(Duration::from_secs(fields.created_at_unix))
        .context("invalid proposal time")?;
    ensure!(
        SystemTime::now()
            .duration_since(created)
            .is_ok_and(|age| age <= MAX_AGE),
        "proposal expired or future-dated"
    );
    let diagnostic = Diagnostic {
        file: Some(fields.path.into()),
        line: Some(fields.diagnostic_line),
        source: None,
        code: None,
        message: String::new(),
        column: None,
    };
    let target = Target::read(&root, &diagnostic)?;
    ensure!(
        rollback::encode_hex(&Sha256::digest(target.original.as_bytes())) == fields.original_sha256,
        "proposal source changed; generate and review a fresh proposal"
    );
    let patch = target.validate_response(&serde_json::to_string(&Replacement {
        before: fields.before,
        after: fields.after,
    })?)?;
    Ok((target, patch))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, String) {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("main.rs"), "old\n").unwrap();
        let diagnostic = Diagnostic {
            file: Some("main.rs".into()),
            line: Some(1),
            source: None,
            code: None,
            message: String::new(),
            column: None,
        };
        let target = Target::read(root.path(), &diagnostic).unwrap();
        let patch = target
            .validate_response(r#"{"before":"old","after":"new"}"#)
            .unwrap();
        let id = save(&target, &patch).unwrap();
        (root, id)
    }

    #[test]
    fn every_saved_field_and_record_identity_is_authenticated() {
        let (root, id) = fixture();
        let path = root
            .path()
            .join(".lbc/proposals")
            .join(format!("{id}.json"));
        let original: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        for (field, value) in [
            ("version", serde_json::json!(2)),
            ("id", serde_json::json!("proposal-other")),
            ("root", serde_json::json!("/other")),
            ("path", serde_json::json!("other.rs")),
            ("diagnostic_line", serde_json::json!(2)),
            ("original_sha256", serde_json::json!("00")),
            ("before", serde_json::json!("different")),
            ("after", serde_json::json!("tampered")),
            ("created_at_unix", serde_json::json!(0)),
        ] {
            let mut tampered = original.clone();
            tampered["fields"][field] = value;
            fs::write(&path, serde_json::to_vec(&tampered).unwrap()).unwrap();
            assert!(load(root.path(), &id).is_err(), "accepted tampered {field}");
            assert_eq!(
                fs::read_to_string(root.path().join("main.rs")).unwrap(),
                "old\n"
            );
        }
        fs::write(&path, serde_json::to_vec(&original).unwrap()).unwrap();
        fs::copy(&path, path.with_file_name("proposal-renamed.json")).unwrap();
        assert!(load(root.path(), "proposal-renamed").is_err());
        assert!(load(root.path(), "../proposal-escape").is_err());
        assert!(load(root.path(), &id).is_ok());
        fs::write(root.path().join("main.rs"), "old\nchanged elsewhere\n").unwrap();
        assert!(load(root.path(), &id).is_err());
    }

    #[test]
    fn authenticated_proposal_cannot_cross_project_or_changed_ignore_boundary() {
        let (root, id) = fixture();
        let other = tempfile::tempdir().unwrap();
        fs::write(other.path().join("main.rs"), "old\n").unwrap();
        fs::create_dir(other.path().join(".lbc")).unwrap();
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(other.path().join(".lbc/proposals")).unwrap();
        for relative in [format!("proposals/{id}.json"), "recovery.key".into()] {
            fs::copy(
                root.path().join(".lbc").join(&relative),
                other.path().join(".lbc").join(&relative),
            )
            .unwrap();
        }
        assert!(load(other.path(), &id).is_err());
        fs::write(root.path().join(".gitignore"), "main.rs\n").unwrap();
        assert!(load(root.path(), &id).is_err());
    }

    #[test]
    fn expired_records_and_retention_are_bounded() {
        let (root, id) = fixture();
        let directory = root.path().join(".lbc/proposals");
        let path = directory.join(format!("{id}.json"));
        let mut record: Record = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        record.fields.created_at_unix = 0;
        let key = rollback::read_recovery_key(&root.path().join(".lbc")).unwrap();
        record.authentication =
            rollback::encode_hex(&mac(&record.fields, &key).unwrap().finalize().into_bytes());
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert!(load(root.path(), &id).is_err());
        let old_time = SystemTime::now() - MAX_AGE - Duration::from_secs(10);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(old_time))
            .unwrap();
        let diagnostic = Diagnostic {
            file: Some("main.rs".into()),
            line: Some(1),
            source: None,
            code: None,
            message: String::new(),
            column: None,
        };
        let target = Target::read(root.path(), &diagnostic).unwrap();
        let patch = target
            .validate_response(r#"{"before":"old","after":"new"}"#)
            .unwrap();
        for _ in 0..101 {
            save(&target, &patch).unwrap();
        }
        assert!(!path.exists());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 100);
        assert_eq!(
            fs::read_to_string(root.path().join("main.rs")).unwrap(),
            "old\n"
        );
    }

    #[cfg(unix)]
    #[test]
    fn public_storage_or_symlinked_source_is_rejected() {
        use std::os::unix::fs::PermissionsExt;
        let (root, id) = fixture();
        let directory = root.path().join(".lbc/proposals");
        let path = directory.join(format!("{id}.json"));
        assert_eq!(
            fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load(root.path(), &id).is_err());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(load(root.path(), &id).is_err());
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        fs::rename(root.path().join("main.rs"), root.path().join("real.rs")).unwrap();
        std::os::unix::fs::symlink("real.rs", root.path().join("main.rs")).unwrap();
        assert!(load(root.path(), &id).is_err());
    }
}
