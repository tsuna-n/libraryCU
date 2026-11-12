use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use super::loader::parse_document;

const MANIFEST_FILE: &str = "package.toml";
const CHECKSUM_FILE: &str = "SHA256SUMS";
const MAX_CHECKSUM_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub description: String,
    pub documents: usize,
    pub integrity: String,
    pub path: PathBuf,
}

pub fn parse_manifest(content: &str) -> Result<PackageManifest> {
    let manifest: PackageManifest =
        toml::from_str(content).context("invalid package.toml manifest")?;
    if !is_valid_package_name(&manifest.name) {
        bail!(
            "package name {:?} is invalid; use lowercase letters, digits, '-' and '_' only",
            manifest.name
        );
    }
    if manifest.version.trim().is_empty() {
        bail!("package version must not be empty");
    }
    Ok(manifest)
}

pub fn is_valid_package_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || matches!(character, '0'..='9' | '-' | '_')
        })
}

/// User-level knowledge directory: `$XDG_DATA_HOME/lbc/knowledge` by default.
pub fn data_dir() -> PathBuf {
    user_data_dir().join("knowledge")
}

pub(crate) fn user_data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("lbc");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/lbc");
    }
    let legacy = PathBuf::from(".local/share/lbc");
    // Preserve existing stores; fresh Windows profiles need stable paths across projects.
    if matches!(fs::symlink_metadata(&legacy), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
        && let Some(profile) = std::env::var_os("USERPROFILE")
    {
        return PathBuf::from(profile).join(".local/share/lbc");
    }
    legacy
}

/// Validate a package directory and publish a checksummed snapshot.
/// Only markdown documents, the manifest, and generated checksums are installed.
pub fn install_package(source: &Path, data_dir: &Path) -> Result<InstalledPackage> {
    install_package_with_hook(source, data_dir, || Ok(()))
}

/// Packages available offline from the binary, without a source checkout.
pub fn bundled_packages() -> Vec<PackageManifest> {
    vec![parse_manifest(super::bundled::MANIFEST).expect("valid bundled package manifest")]
}

pub fn install_bundled_package(name: &str, data_dir: &Path) -> Result<InstalledPackage> {
    let manifest = parse_manifest(super::bundled::MANIFEST)?;
    if name != manifest.name {
        bail!("unknown bundled package {name:?}");
    }
    let documents: Vec<_> = super::bundled::DOCUMENTS
        .iter()
        .map(|(path, content)| ((*path).to_owned(), (*content).to_owned()))
        .collect();
    validate_documents(&documents)?;
    install_snapshot(
        super::bundled::MANIFEST,
        manifest,
        &documents,
        data_dir,
        || Ok(()),
    )
}

fn install_package_with_hook<F>(
    source: &Path,
    data_dir: &Path,
    before_publish: F,
) -> Result<InstalledPackage>
where
    F: FnOnce() -> Result<()>,
{
    crate::security::files::reject_symlinks(source)?;
    crate::security::files::reject_symlinks(data_dir)?;
    let manifest_path = source.join(MANIFEST_FILE);
    let manifest_content = crate::security::files::read_text(&manifest_path, 256 * 1024)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest = parse_manifest(&manifest_content)?;

    let documents = collect_documents(source)?;
    if documents.is_empty() {
        bail!("package contains no markdown knowledge documents");
    }

    let checksum_path = source.join(CHECKSUM_FILE);
    match fs::symlink_metadata(&checksum_path) {
        Ok(_) => {
            let checksums = crate::security::files::read_text(&checksum_path, MAX_CHECKSUM_BYTES)?;
            validate_installed_tree(source)?;
            verify_checksums(
                &checksums,
                &package_checksums(&manifest_content, &documents),
            )
            .context("incoming package checksum verification failed")?;
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("failed to inspect package checksum manifest"),
    }

    install_snapshot(
        &manifest_content,
        manifest,
        &documents,
        data_dir,
        before_publish,
    )
}

fn install_snapshot<F>(
    manifest_content: &str,
    manifest: PackageManifest,
    documents: &[(String, String)],
    data_dir: &Path,
    before_publish: F,
) -> Result<InstalledPackage>
where
    F: FnOnce() -> Result<()>,
{
    crate::security::files::reject_symlinks(data_dir)?;

    crate::security::permissions::validate_directory(data_dir)?;
    fs::create_dir_all(data_dir)
        .with_context(|| format!("failed to create {}", data_dir.display()))?;
    let _lock = crate::security::storage::lock_exclusive(&data_dir.join(".packages.lock"))?;
    let target = data_dir.join(&manifest.name);
    if target.exists() {
        bail!(
            "package {:?} is already installed at {}; remove it first",
            manifest.name,
            target.display()
        );
    }
    let staging = tempfile::Builder::new()
        .prefix(".package-install-")
        .tempdir_in(data_dir)
        .with_context(|| format!("failed to stage package in {}", data_dir.display()))?;
    write_synced(
        staging.path().join(MANIFEST_FILE),
        manifest_content.as_bytes(),
    )
    .with_context(|| format!("failed to stage manifest for {:?}", manifest.name))?;
    for (relative, content) in documents {
        let destination = staging.path().join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        // Write the validated snapshot; reopening the source could copy a
        // different (or now symlinked) file after validation.
        write_synced(&destination, content.as_bytes())
            .with_context(|| format!("failed to copy {relative}"))?;
    }
    let checksums = package_checksums(manifest_content, documents);
    write_synced(
        staging.path().join(CHECKSUM_FILE),
        encode_checksums(&checksums).as_bytes(),
    )
    .context("failed to write package integrity manifest")?;
    verify_package_integrity(staging.path())?;
    sync_staged_directories(staging.path())?;
    before_publish()?;
    crate::security::storage::atomic_publish_directory(staging, &target)?;

    Ok(InstalledPackage {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        documents: documents.len(),
        integrity: "verified".to_owned(),
        path: target,
    })
}

pub fn list_packages(data_dir: &Path) -> Vec<InstalledPackage> {
    let mut packages = Vec::new();
    if crate::security::files::reject_symlinks(data_dir).is_err() {
        return packages;
    }
    let Ok(entries) = fs::read_dir(data_dir) else {
        return packages;
    };
    for entry in entries.flatten() {
        let entry_name = entry.file_name().to_string_lossy().into_owned();
        if !entry.path().is_dir() || !is_valid_package_name(&entry_name) {
            continue;
        }
        let manifest =
            crate::security::files::read_text(&entry.path().join(MANIFEST_FILE), 256 * 1024)
                .and_then(|content| parse_manifest(&content))
                .and_then(|manifest| {
                    if manifest.name != entry_name {
                        bail!("package manifest name does not match its directory");
                    }
                    Ok(manifest)
                });
        let Ok(manifest) = manifest else {
            packages.push(InstalledPackage {
                name: entry_name,
                version: "unknown".to_owned(),
                description: String::new(),
                documents: 0,
                integrity: "corrupt".to_owned(),
                path: entry.path(),
            });
            continue;
        };
        let (documents, integrity) = if package_has_integrity_manifest(&entry.path()) {
            match verify_package_integrity(&entry.path()) {
                Ok(documents) => (documents, "verified"),
                Err(_) => (0, "corrupt"),
            }
        } else {
            match collect_documents(&entry.path()) {
                Ok(documents) => (documents.len(), "unverified"),
                Err(_) => (0, "corrupt"),
            }
        };
        packages.push(InstalledPackage {
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            documents,
            integrity: integrity.to_owned(),
            path: entry.path(),
        });
    }
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    packages
}

pub fn package_has_integrity_manifest(package_root: &Path) -> bool {
    !matches!(
        fs::symlink_metadata(package_root.join(CHECKSUM_FILE)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    )
}

/// Remove an installed package and return its former path.
pub fn remove_package(name: &str, data_dir: &Path) -> Result<PathBuf> {
    if !is_valid_package_name(name) {
        bail!("invalid package name {name:?}");
    }
    let _lock = crate::security::storage::lock_exclusive(&data_dir.join(".packages.lock"))?;
    let target = data_dir.join(name);
    crate::security::files::reject_symlinks(&target)?;
    if !target.is_dir() {
        bail!(
            "package {name:?} is not installed in {}",
            data_dir.display()
        );
    }
    // Inspect the entire tree before deletion so a foreign-owned/ACL-granted
    // descendant cannot be silently treated as trusted mutable storage.
    for entry in WalkDir::new(&target).follow_links(false) {
        let entry = entry?;
        crate::security::permissions::validate_path(entry.path(), false)?;
    }
    fs::remove_dir_all(&target)
        .with_context(|| format!("failed to remove {}", target.display()))?;
    Ok(target)
}

/// Verify the generated integrity manifest and return the number of documents.
pub fn verify_package_integrity(package_root: &Path) -> Result<usize> {
    crate::security::files::reject_symlinks(package_root)?;
    let installed_paths = validate_installed_tree(package_root)?;
    let checksum_path = package_root.join(CHECKSUM_FILE);
    let content = crate::security::files::read_text(&checksum_path, MAX_CHECKSUM_BYTES)
        .with_context(|| {
            format!(
                "missing or unreadable package checksum manifest at {}",
                checksum_path.display()
            )
        })?;
    let documents = collect_documents(package_root)?;
    let manifest =
        crate::security::files::read_text(&package_root.join(MANIFEST_FILE), 256 * 1024)?;
    let expected = package_checksums(&manifest, &documents);
    if expected.keys().cloned().collect::<BTreeSet<_>>() != installed_paths {
        bail!("package checksum manifest does not match installed package files");
    }
    verify_checksums(&content, &expected)?;
    Ok(documents.len())
}

fn verify_checksums(content: &str, expected: &BTreeMap<String, String>) -> Result<()> {
    let mut declared = BTreeMap::new();
    for (index, line) in content.lines().enumerate() {
        let (digest, relative) = line
            .split_once("  ")
            .with_context(|| format!("invalid checksum line {}", index + 1))?;
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            bail!("invalid SHA-256 digest on checksum line {}", index + 1);
        }
        let relative = Path::new(relative);
        if relative.as_os_str().is_empty()
            || !relative
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_)))
        {
            bail!("unsafe checksum path on line {}", index + 1);
        }
        let key = relative.to_string_lossy().replace('\\', "/");
        if declared
            .insert(key.clone(), digest.to_ascii_lowercase())
            .is_some()
        {
            bail!("duplicate checksum entry for {key}");
        }
    }

    let expected_paths: BTreeSet<_> = expected.keys().cloned().collect();
    let declared_paths: BTreeSet<_> = declared.keys().cloned().collect();
    if expected_paths != declared_paths {
        bail!("package checksum manifest does not match installed package files");
    }
    for (path, digest) in expected {
        if declared.get(path) != Some(digest) {
            bail!("package checksum mismatch for {path}");
        }
    }
    Ok(())
}

fn validate_installed_tree(package_root: &Path) -> Result<BTreeSet<String>> {
    let mut paths = BTreeSet::new();
    for entry in WalkDir::new(package_root).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to walk {}", package_root.display()))?;
        if entry.depth() == 0 || entry.file_type().is_dir() {
            continue;
        }
        if !entry.file_type().is_file() {
            bail!(
                "package contains an unsupported file: {}",
                entry.path().display()
            );
        }
        let relative = entry
            .path()
            .strip_prefix(package_root)
            .context("package file escaped its root")?;
        let relative = relative
            .to_str()
            .filter(|path| !path.contains(['\n', '\r']))
            .context("package file path must be valid UTF-8 without newlines")?
            .replace('\\', "/");
        if relative == CHECKSUM_FILE {
            continue;
        }
        if relative != MANIFEST_FILE && !relative.ends_with(".md") {
            bail!("unexpected installed package file {relative}");
        }
        paths.insert(relative);
    }
    Ok(paths)
}

fn package_checksums(manifest: &str, documents: &[(String, String)]) -> BTreeMap<String, String> {
    let mut checksums = BTreeMap::new();
    checksums.insert(MANIFEST_FILE.to_owned(), sha256_hex(manifest.as_bytes()));
    for (relative, content) in documents {
        checksums.insert(relative.replace('\\', "/"), sha256_hex(content.as_bytes()));
    }
    checksums
}

fn encode_checksums(checksums: &BTreeMap<String, String>) -> String {
    checksums
        .iter()
        .map(|(path, digest)| format!("{digest}  {path}\n"))
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Collect and validate every markdown document below the package root.
fn collect_documents(package_root: &Path) -> Result<Vec<(String, String)>> {
    crate::security::files::reject_symlinks(package_root)?;
    let mut documents = Vec::new();
    for entry in WalkDir::new(package_root).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to walk {}", package_root.display()))?;
        if entry.file_type().is_symlink() {
            bail!("package contains a symlink: {}", entry.path().display());
        }
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some("md")
        {
            continue;
        }
        let content = crate::security::files::read_text(entry.path(), 256 * 1024)
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        let relative_path = entry
            .path()
            .strip_prefix(package_root)
            .unwrap_or(entry.path());
        if !relative_path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
        {
            bail!("unsafe package document path {}", entry.path().display());
        }
        let relative = relative_path
            .to_str()
            .filter(|path| !path.contains(['\n', '\r']))
            .context("package document path must be valid UTF-8 without newlines")?
            .to_owned();
        documents.push((relative, content));
    }
    documents.sort();
    validate_documents(&documents)?;
    Ok(documents)
}

fn validate_documents(documents: &[(String, String)]) -> Result<()> {
    let mut ids = HashSet::new();
    let mut paths = HashSet::new();
    for (relative, content) in documents {
        if !paths.insert(relative.replace('\\', "/").to_lowercase()) {
            bail!("duplicate package document path {relative:?}");
        }
        let document = parse_document(relative, content)
            .with_context(|| format!("invalid knowledge document {relative}"))?;
        if !ids.insert(document.metadata.id.to_ascii_lowercase()) {
            bail!(
                "duplicate knowledge ID {:?} in package",
                document.metadata.id
            );
        }
    }
    Ok(())
}

fn write_synced(path: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let mut file = fs::File::create(path)?;
    crate::security::permissions::validate_file(&file, false)?;
    use std::io::Write;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn sync_staged_directories(root: &Path) -> Result<()> {
    let mut directories = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for directory in directories {
        crate::security::storage::sync_directory(&directory)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        std::env::temp_dir().join(format!("lbc-pkg-{name}-{}-{nonce}", std::process::id()))
    }

    fn write_package(root: &Path) {
        fs::create_dir_all(root).unwrap();
        fs::write(
            root.join("package.toml"),
            "name = \"demo-pack\"\nversion = \"0.1.0\"\ndescription = \"demo\"\n",
        )
        .unwrap();
        fs::write(
            root.join("doc.md"),
            "---\nid: demo-doc\ntags:\n  - demo\n---\n# Demo\n\nBody.\n",
        )
        .unwrap();
    }

    #[test]
    fn bundled_package_installs_all_embedded_documents_and_rejects_duplicates() {
        let data = tempfile::tempdir().unwrap();
        let available = bundled_packages();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].name, "python-fastapi-basics");
        let installed = install_bundled_package(&available[0].name, data.path()).unwrap();
        assert_eq!(installed.documents, 40);
        assert_eq!(verify_package_integrity(&installed.path).unwrap(), 40);
        assert!(installed.path.join("fastapi/minimal-app.md").is_file());
        assert!(install_bundled_package(&available[0].name, data.path()).is_err());
        assert!(install_bundled_package("../unknown", data.path()).is_err());
        assert!(install_bundled_package("unknown", data.path()).is_err());
    }

    #[test]
    fn install_verifies_incoming_checksums_before_publishing() {
        let source = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        write_package(source.path());
        let manifest = fs::read_to_string(source.path().join(MANIFEST_FILE)).unwrap();
        let documents = collect_documents(source.path()).unwrap();
        let checksums = encode_checksums(&package_checksums(&manifest, &documents));
        for invalid in [
            checksums.replace(&sha256_hex(documents[0].1.as_bytes()), &"0".repeat(64)),
            format!("{checksums}{checksums}"),
            "not a checksum\n".to_owned(),
        ] {
            fs::write(source.path().join(CHECKSUM_FILE), invalid).unwrap();
            assert!(install_package(source.path(), data.path()).is_err());
            assert!(!data.path().join("demo-pack").exists());
        }
        fs::write(source.path().join(CHECKSUM_FILE), checksums).unwrap();
        fs::write(source.path().join("unexpected.bin"), "undeclared").unwrap();
        assert!(install_package(source.path(), data.path()).is_err());
        fs::remove_file(source.path().join("unexpected.bin")).unwrap();
        assert!(install_package(source.path(), data.path()).is_ok());
    }

    #[test]
    fn package_document_names_and_ids_must_be_unique_ignoring_case() {
        let content = "---\nid: demo\n---\nBody\n";
        let other = "---\nid: DEMO\n---\nBody\n";
        assert!(
            validate_documents(&[
                ("first.md".to_owned(), content.to_owned()),
                ("second.md".to_owned(), other.to_owned()),
            ])
            .is_err()
        );
        assert!(
            validate_documents(&[
                ("First.md".to_owned(), content.to_owned()),
                (
                    "first.md".to_owned(),
                    "---\nid: other\n---\nBody\n".to_owned()
                ),
            ])
            .is_err()
        );
    }

    #[test]
    fn non_file_checksum_manifest_is_corrupt_rather_than_unverified() {
        let data = tempfile::tempdir().unwrap();
        let package = data.path().join("demo-pack");
        write_package(&package);
        fs::create_dir(package.join(CHECKSUM_FILE)).unwrap();
        assert_eq!(list_packages(data.path())[0].integrity, "corrupt");
    }

    #[test]
    fn list_marks_manifest_directory_aliases_corrupt() {
        let data = tempfile::tempdir().unwrap();
        write_package(&data.path().join("alias"));
        let packages = list_packages(data.path());
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "alias");
        assert_eq!(packages[0].integrity, "corrupt");
        assert_eq!(packages[0].documents, 0);
    }

    #[test]
    fn install_list_remove_roundtrip() {
        let source = temp_dir("source");
        let data = temp_dir("data");
        write_package(&source);

        let installed = install_package(&source, &data).expect("install should succeed");
        assert_eq!(installed.name, "demo-pack");
        assert_eq!(installed.documents, 1);
        assert!(data.join("demo-pack/doc.md").exists());
        fs::create_dir(data.join(".package-install-stale")).unwrap();

        let listed = list_packages(&data);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "demo-pack");

        let removed = remove_package("demo-pack", &data).expect("remove should succeed");
        assert_eq!(removed, data.join("demo-pack"));
        assert!(list_packages(&data).is_empty());

        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn rejects_duplicate_and_traversal_names() {
        let source = temp_dir("dup");
        let data = temp_dir("dup-data");
        write_package(&source);
        install_package(&source, &data).expect("first install should succeed");
        assert!(
            install_package(&source, &data).is_err(),
            "duplicate rejected"
        );

        assert!(is_valid_package_name("rust-core"));
        assert!(!is_valid_package_name("../evil"));
        assert!(!is_valid_package_name(""));
        assert!(remove_package("../evil", &data).is_err());

        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn install_rejects_packages_with_invalid_documents() {
        let source = temp_dir("invalid");
        let data = temp_dir("invalid-data");
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("package.toml"),
            "name = \"broken-pack\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(source.join("bad.md"), "no frontmatter here\n").unwrap();
        assert!(install_package(&source, &data).is_err());
        assert!(!data.join("broken-pack").exists());
        fs::remove_dir_all(source).unwrap();
        if data.exists() {
            fs::remove_dir_all(data).unwrap();
        }
    }

    #[test]
    fn installed_package_tampering_is_detected() {
        let source = temp_dir("tamper-source");
        let data = temp_dir("tamper-data");
        write_package(&source);
        let installed = install_package(&source, &data).expect("install should succeed");
        assert_eq!(verify_package_integrity(&installed.path).unwrap(), 1);

        fs::write(installed.path.join("unexpected.bin"), b"not declared").unwrap();
        assert!(verify_package_integrity(&installed.path).is_err());
        fs::remove_file(installed.path.join("unexpected.bin")).unwrap();

        let checksum_path = installed.path.join(CHECKSUM_FILE);
        let checksums = fs::read_to_string(&checksum_path).unwrap();
        fs::write(&checksum_path, "not-a-digest  doc.md\n").unwrap();
        assert!(verify_package_integrity(&installed.path).is_err());
        fs::write(&checksum_path, format!("{checksums}{checksums}")).unwrap();
        assert!(verify_package_integrity(&installed.path).is_err());
        fs::write(
            &checksum_path,
            checksums
                .lines()
                .filter(|line| !line.ends_with("doc.md"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
        assert!(verify_package_integrity(&installed.path).is_err());
        fs::write(&checksum_path, &checksums).unwrap();

        #[cfg(unix)]
        {
            let outside = source.join("outside.md");
            fs::write(&outside, "outside").unwrap();
            fs::remove_file(installed.path.join("doc.md")).unwrap();
            std::os::unix::fs::symlink(&outside, installed.path.join("doc.md")).unwrap();
            assert!(verify_package_integrity(&installed.path).is_err());
            fs::remove_file(installed.path.join("doc.md")).unwrap();
            fs::write(
                installed.path.join("doc.md"),
                "---\nid: demo-doc\ntags:\n  - demo\n---\n# Demo\n\nBody.\n",
            )
            .unwrap();
        }

        fs::write(installed.path.join("doc.md"), "tampered").unwrap();
        assert!(verify_package_integrity(&installed.path).is_err());
        let listed = list_packages(&data);
        assert_eq!(listed[0].integrity, "corrupt");
        assert_eq!(listed[0].documents, 0);

        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn concurrent_installs_publish_one_complete_package() {
        let source = temp_dir("concurrent-source");
        let data = temp_dir("concurrent-data");
        write_package(&source);
        let source_one = source.clone();
        let data_one = data.clone();
        let source_two = source.clone();
        let data_two = data.clone();

        let first = std::thread::spawn(move || install_package(&source_one, &data_one));
        let second = std::thread::spawn(move || install_package(&source_two, &data_two));
        let results = [first.join().unwrap(), second.join().unwrap()];

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(list_packages(&data).len(), 1);
        assert_eq!(
            verify_package_integrity(&data.join("demo-pack")).unwrap(),
            1
        );
        assert_eq!(
            fs::read_dir(&data)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".package-install-"))
                .count(),
            0
        );

        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn failure_before_publish_leaves_no_partial_package() {
        let source = temp_dir("failure-source");
        let data = temp_dir("failure-data");
        write_package(&source);

        let result = install_package_with_hook(&source, &data, || {
            bail!("injected package publication failure")
        });

        assert!(result.is_err());
        assert!(!data.join("demo-pack").exists());
        assert_eq!(
            fs::read_dir(&data)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".package-install-"))
                .count(),
            0
        );
        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn target_created_during_install_is_preserved_without_partial_publication() {
        let source = temp_dir("target-race-source");
        let data = temp_dir("target-race-data");
        write_package(&source);

        let result = install_package_with_hook(&source, &data, || {
            fs::create_dir(data.join("demo-pack"))?;
            fs::write(data.join("demo-pack/concurrent"), "newer owner")?;
            Ok(())
        });

        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(data.join("demo-pack/concurrent")).unwrap(),
            "newer owner"
        );
        assert!(!data.join("demo-pack/doc.md").exists());
        assert_eq!(
            fs::read_dir(&data)
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".package-install-"))
                .count(),
            0
        );

        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(data).unwrap();
    }

    #[test]
    fn source_mutation_after_validation_cannot_change_the_staged_snapshot() {
        let source = temp_dir("source-race-source");
        let data = temp_dir("source-race-data");
        write_package(&source);

        let installed = install_package_with_hook(&source, &data, || {
            fs::write(source.join("doc.md"), "mutated after validation")?;
            Ok(())
        })
        .expect("validated snapshot should publish");

        assert_eq!(verify_package_integrity(&installed.path).unwrap(), 1);
        assert!(
            fs::read_to_string(installed.path.join("doc.md"))
                .unwrap()
                .contains("Body.")
        );
        assert_eq!(
            fs::read_to_string(source.join("doc.md")).unwrap(),
            "mutated after validation"
        );

        fs::remove_dir_all(source).unwrap();
        fs::remove_dir_all(data).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn install_rejects_symlinked_package_content() {
        let source = temp_dir("symlink-source");
        let data = temp_dir("symlink-data");
        write_package(&source);
        std::os::unix::fs::symlink(source.join("doc.md"), source.join("linked.md")).unwrap();

        assert!(install_package(&source, &data).is_err());
        assert!(!data.join("demo-pack").exists());

        fs::remove_dir_all(source).unwrap();
        if data.exists() {
            fs::remove_dir_all(data).unwrap();
        }
    }
}
