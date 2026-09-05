use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use super::loader::parse_document;

const MANIFEST_FILE: &str = "package.toml";

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
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path).join("lbc/knowledge");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".local/share/lbc/knowledge");
    }
    PathBuf::from(".local/share/lbc/knowledge")
}

/// Validate a package directory and copy it into the data directory.
/// Only markdown documents and the manifest are copied.
pub fn install_package(source: &Path, data_dir: &Path) -> Result<InstalledPackage> {
    let manifest_path = source.join(MANIFEST_FILE);
    let manifest_content = fs::read_to_string(&manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let manifest = parse_manifest(&manifest_content)?;

    let documents = collect_documents(source)?;
    if documents.is_empty() {
        bail!("package contains no markdown knowledge documents");
    }

    let target = data_dir.join(&manifest.name);
    if target.exists() {
        bail!(
            "package {:?} is already installed at {}; remove it first",
            manifest.name,
            target.display()
        );
    }
    fs::create_dir_all(&target)
        .with_context(|| format!("failed to create {}", target.display()))?;
    fs::write(target.join(MANIFEST_FILE), &manifest_content)
        .with_context(|| format!("failed to write the manifest into {}", target.display()))?;
    for (relative, _content) in &documents {
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::copy(source.join(relative), &destination)
            .with_context(|| format!("failed to copy {relative}"))?;
    }

    Ok(InstalledPackage {
        name: manifest.name,
        version: manifest.version,
        description: manifest.description,
        documents: documents.len(),
        path: target,
    })
}

pub fn list_packages(data_dir: &Path) -> Vec<InstalledPackage> {
    let mut packages = Vec::new();
    let Ok(entries) = fs::read_dir(data_dir) else {
        return packages;
    };
    for entry in entries.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let Ok(content) = fs::read_to_string(entry.path().join(MANIFEST_FILE)) else {
            continue;
        };
        let Ok(manifest) = parse_manifest(&content) else {
            continue;
        };
        let documents = match collect_documents(&entry.path()) {
            Ok(documents) => documents.len(),
            Err(_) => 0,
        };
        packages.push(InstalledPackage {
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            documents,
            path: entry.path(),
        });
    }
    packages.sort_by(|left, right| left.name.cmp(&right.name));
    packages
}

/// Remove an installed package and return its former path.
pub fn remove_package(name: &str, data_dir: &Path) -> Result<PathBuf> {
    if !is_valid_package_name(name) {
        bail!("invalid package name {name:?}");
    }
    let target = data_dir.join(name);
    if !target.is_dir() {
        bail!(
            "package {name:?} is not installed in {}",
            data_dir.display()
        );
    }
    fs::remove_dir_all(&target)
        .with_context(|| format!("failed to remove {}", target.display()))?;
    Ok(target)
}

/// Collect and validate every markdown document below the package root.
fn collect_documents(package_root: &Path) -> Result<Vec<(String, String)>> {
    let mut documents = Vec::new();
    let mut ids = HashSet::new();
    for entry in WalkDir::new(package_root).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to walk {}", package_root.display()))?;
        if !entry.file_type().is_file()
            || entry.path().extension().and_then(|ext| ext.to_str()) != Some("md")
        {
            continue;
        }
        let content = fs::read_to_string(entry.path())
            .with_context(|| format!("failed to read {}", entry.path().display()))?;
        let relative = entry
            .path()
            .strip_prefix(package_root)
            .unwrap_or(entry.path())
            .to_string_lossy()
            .into_owned();
        // Validate the document parses; failures abort the whole install.
        let document = parse_document(&relative, &content)
            .with_context(|| format!("invalid knowledge document {}", entry.path().display()))?;
        if !ids.insert(document.metadata.id.clone()) {
            bail!(
                "duplicate knowledge ID {:?} in package",
                document.metadata.id
            );
        }
        documents.push((relative, content));
    }
    documents.sort();
    Ok(documents)
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
    fn install_list_remove_roundtrip() {
        let source = temp_dir("source");
        let data = temp_dir("data");
        write_package(&source);

        let installed = install_package(&source, &data).expect("install should succeed");
        assert_eq!(installed.name, "demo-pack");
        assert_eq!(installed.documents, 1);
        assert!(data.join("demo-pack/doc.md").exists());

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
        fs::create_dir_all(&source).unwrap();
        fs::write(
            source.join("package.toml"),
            "name = \"broken-pack\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::write(source.join("bad.md"), "no frontmatter here\n").unwrap();
        assert!(install_package(&source, &temp_dir("invalid-data")).is_err());
        fs::remove_dir_all(source).unwrap();
    }
}
