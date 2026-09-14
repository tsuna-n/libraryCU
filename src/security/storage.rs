use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result};
use fs2::FileExt;

/// Holds an exclusive advisory lock for a mutable LBC store.
pub struct StoreLock {
    file: fs::File,
}

impl Drop for StoreLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

pub fn lock_exclusive(path: &Path) -> Result<StoreLock> {
    super::files::reject_symlinks(path)?;
    let parent = path.parent().context("lock path has no parent")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create lock directory {}", parent.display()))?;
    super::files::reject_symlinks(path)?;

    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .with_context(|| format!("failed to open store lock {}", path.display()))?;
    file.lock_exclusive()
        .with_context(|| format!("failed to lock mutable store at {}", path.display()))?;
    Ok(StoreLock { file })
}

/// Replace a regular file from a fully written temporary file in the same directory.
pub fn atomic_replace(path: &Path, bytes: &[u8], private: bool) -> Result<()> {
    super::files::reject_symlinks(path)?;
    let parent = path.parent().context("target path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    super::files::reject_symlinks(path)?;

    let mut temp = tempfile::Builder::new()
        .prefix(".lbc-write-")
        .tempfile_in(parent)?;
    if private {
        let mut permissions = temp.as_file().metadata()?.permissions();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            permissions.set_mode(0o600);
        }
        temp.as_file().set_permissions(permissions)?;
    } else if let Ok(metadata) = fs::metadata(path) {
        temp.as_file().set_permissions(metadata.permissions())?;
    }
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    super::files::reject_symlinks(path)?;
    temp.persist(path)
        .with_context(|| format!("failed to replace {} atomically", path.display()))?;
    sync_directory(parent)?;
    Ok(())
}

pub(crate) fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        fs::File::open(path)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
