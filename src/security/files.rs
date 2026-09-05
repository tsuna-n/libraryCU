use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

/// Inspect every existing component, including parents of a not-yet-created
/// store. A final-component check alone misses .lbc -> another directory.
pub fn reject_symlinks(path: &Path) -> Result<()> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut current = PathBuf::new();
    for component in absolute.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                bail!("refusing symlinked path {}", current.display());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to inspect {}", current.display()));
            }
        }
    }
    Ok(())
}

/// Bound reads on the opened file, not only on earlier path metadata. Refuse
/// special files before opening so a FIFO/device cannot hang an analysis.
pub fn read_text(path: &Path, max_bytes: u64) -> Result<String> {
    reject_symlinks(path)?;
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("failed to inspect {}", path.display()))?;
    if !metadata.is_file() {
        bail!("expected a regular file: {}", path.display());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        bail!("expected a regular file: {}", path.display());
    }
    if metadata.len() > max_bytes {
        bail!("file exceeds {max_bytes} bytes: {}", path.display());
    }
    let mut content = String::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_string(&mut content)
        .with_context(|| format!("failed to read UTF-8 text from {}", path.display()))?;
    if content.len() as u64 > max_bytes {
        bail!("file exceeds {max_bytes} bytes: {}", path.display());
    }
    Ok(content)
}
