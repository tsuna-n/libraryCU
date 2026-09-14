use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result, bail};
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
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options
        .open(path)
        .with_context(|| format!("failed to open store lock {}", path.display()))?;
    file.lock_exclusive()
        .with_context(|| format!("failed to lock mutable store at {}", path.display()))?;
    Ok(StoreLock { file })
}

/// Replace a regular file from a fully written temporary file in the same directory.
///
/// Unix uses directory-relative file descriptors for the final validation and
/// rename, so swapping an inspected parent for a symlink cannot redirect the
/// write. Other platforms retain the portable tempfile implementation and the
/// existing component checks.
pub fn atomic_replace(path: &Path, bytes: &[u8], private: bool) -> Result<()> {
    atomic_replace_with_hook(path, bytes, private, || Ok(()))
}

#[cfg(unix)]
fn atomic_replace_with_hook<F>(path: &Path, bytes: &[u8], private: bool, hook: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    use std::{
        ffi::{CStr, CString},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        },
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    super::files::reject_symlinks(path)?;
    let parent = path.parent().context("target path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    super::files::reject_symlinks(path)?;

    let parent_file = open_directory_no_symlinks(parent)?;
    let parent_identity = file_identity(&parent_file)?;
    let target_name = cstring(
        path.file_name()
            .context("target path has no file name")?
            .as_bytes(),
    )?;
    let original = stat_at(&parent_file, &target_name)?;
    if let Some(stat) = &original
        && (stat.st_mode & libc::S_IFMT) != libc::S_IFREG
    {
        bail!("refusing to replace a non-regular file: {}", path.display());
    }

    let mut temporary = None;
    for _ in 0..128 {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!(".lbc-write-{}-{sequence}.tmp", std::process::id()))?;
        // SAFETY: parent_file is an open directory, name is NUL-terminated,
        // and successful descriptors are immediately owned by fs::File.
        let descriptor = unsafe {
            libc::openat(
                parent_file.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                0o600,
            )
        };
        if descriptor >= 0 {
            // SAFETY: openat returned a new owned descriptor.
            let file = unsafe { fs::File::from_raw_fd(descriptor) };
            temporary = Some((name, file));
            break;
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(error).context("failed to create atomic replacement file");
        }
    }
    let (temporary_name, mut temporary_file) =
        temporary.context("could not allocate a unique atomic replacement file")?;
    let mut cleanup = TempAt {
        directory: &parent_file,
        name: &temporary_name,
        published: false,
    };

    if !private && let Some(stat) = &original {
        // SAFETY: the descriptor is valid for the lifetime of temporary_file.
        if unsafe { libc::fchmod(temporary_file.as_raw_fd(), stat.st_mode & 0o7777) } != 0 {
            return Err(std::io::Error::last_os_error())
                .context("failed to preserve target permissions");
        }
    }
    temporary_file.write_all(bytes)?;
    temporary_file.sync_all()?;
    hook()?;

    let current_parent = open_directory_no_symlinks(parent)?;
    if file_identity(&current_parent)? != parent_identity {
        bail!("target directory changed during atomic replacement");
    }
    let current = stat_at(&parent_file, &target_name)?;
    if !same_file_state(original.as_ref(), current.as_ref()) {
        bail!("target changed during atomic replacement");
    }

    // SAFETY: both names are valid C strings and both directory descriptors
    // remain open. renameat replaces the directory entry without following a
    // final-component symlink.
    if unsafe {
        libc::renameat(
            parent_file.as_raw_fd(),
            temporary_name.as_ptr(),
            parent_file.as_raw_fd(),
            target_name.as_ptr(),
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("failed to replace {} atomically", path.display()));
    }
    parent_file.sync_all()?;
    cleanup.published = true;
    return Ok(());

    fn cstring(bytes: &[u8]) -> Result<CString> {
        CString::new(bytes).context("path contains a NUL byte")
    }

    fn open_directory_no_symlinks(path: &Path) -> Result<fs::File> {
        use std::path::Component;

        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()?.join(path)
        };
        let root = CString::new("/")?;
        // SAFETY: root is a valid C string and the returned descriptor is
        // transferred into fs::File on success.
        let root_fd = unsafe {
            libc::open(
                root.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if root_fd < 0 {
            return Err(std::io::Error::last_os_error()).context("failed to open filesystem root");
        }
        // SAFETY: open returned a new owned descriptor.
        let mut directory = unsafe { fs::File::from_raw_fd(root_fd) };
        for component in absolute.components() {
            let Component::Normal(name) = component else {
                if matches!(component, Component::RootDir | Component::CurDir) {
                    continue;
                }
                bail!("directory path contains an unsupported component");
            };
            let name = cstring(name.as_bytes())?;
            // SAFETY: directory is open and name remains alive for the call.
            let descriptor = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                )
            };
            if descriptor < 0 {
                return Err(std::io::Error::last_os_error()).with_context(|| {
                    format!(
                        "failed to open directory without symlinks: {}",
                        path.display()
                    )
                });
            }
            // SAFETY: openat returned a new owned descriptor.
            directory = unsafe { fs::File::from_raw_fd(descriptor) };
        }
        Ok(directory)
    }

    fn stat_at(directory: &fs::File, name: &CStr) -> Result<Option<libc::stat>> {
        let mut stat = std::mem::MaybeUninit::<libc::stat>::zeroed();
        // SAFETY: the pointers are valid and stat points to writable storage.
        let result = unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        };
        if result == 0 {
            // SAFETY: fstatat initialized stat on success.
            return Ok(Some(unsafe { stat.assume_init() }));
        }
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error).context("failed to inspect atomic replacement target")
        }
    }

    fn file_identity(file: &fs::File) -> Result<(u64, u64)> {
        use std::os::unix::fs::MetadataExt;
        let metadata = file.metadata()?;
        Ok((metadata.dev(), metadata.ino()))
    }

    fn same_file_state(before: Option<&libc::stat>, after: Option<&libc::stat>) -> bool {
        match (before, after) {
            (None, None) => true,
            (Some(before), Some(after)) => {
                before.st_dev == after.st_dev
                    && before.st_ino == after.st_ino
                    && before.st_size == after.st_size
            }
            _ => false,
        }
    }

    struct TempAt<'a> {
        directory: &'a fs::File,
        name: &'a CStr,
        published: bool,
    }

    impl Drop for TempAt<'_> {
        fn drop(&mut self) {
            if self.published {
                return;
            }
            // SAFETY: the directory and name outlive this cleanup guard.
            let _ = unsafe { libc::unlinkat(self.directory.as_raw_fd(), self.name.as_ptr(), 0) };
        }
    }
}

#[cfg(not(unix))]
fn atomic_replace_with_hook<F>(path: &Path, bytes: &[u8], private: bool, hook: F) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    super::files::reject_symlinks(path)?;
    let parent = path.parent().context("target path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    super::files::reject_symlinks(path)?;

    let mut temp = tempfile::Builder::new()
        .prefix(".lbc-write-")
        .tempfile_in(parent)?;
    if !private && let Ok(metadata) = fs::metadata(path) {
        temp.as_file().set_permissions(metadata.permissions())?;
    }
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    hook()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn injected_failure_preserves_the_original_and_cleans_up() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("value");
        fs::write(&path, "original")?;

        let result = atomic_replace_with_hook(&path, b"replacement", false, || {
            bail!("injected before publish")
        });

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path)?, "original");
        assert_eq!(fs::read_dir(root.path())?.count(), 1);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn parent_symlink_swap_cannot_redirect_an_atomic_replacement() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let parent = root.path().join("store");
        let moved = root.path().join("store-moved");
        fs::create_dir(&parent)?;
        fs::write(parent.join("value"), "original")?;
        fs::write(outside.path().join("value"), "outside")?;

        let result = atomic_replace_with_hook(&parent.join("value"), b"replacement", false, || {
            fs::rename(&parent, &moved)?;
            symlink(outside.path(), &parent)?;
            Ok(())
        });

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(moved.join("value"))?, "original");
        assert_eq!(fs::read_to_string(outside.path().join("value"))?, "outside");
        Ok(())
    }
}
