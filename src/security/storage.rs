use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result, bail, ensure};
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
    super::permissions::validate_directory(parent)?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create lock directory {}", parent.display()))?;
    super::files::reject_symlinks(path)?;

    super::permissions::validate_directory(parent)?;

    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .with_context(|| format!("failed to open store lock {}", path.display()))?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file(),
        "store lock is not a regular file: {}",
        path.display()
    );
    super::permissions::validate_file(&file, true)?;
    file.lock_exclusive()
        .with_context(|| format!("failed to lock mutable store at {}", path.display()))?;
    super::permissions::validate_directory(parent)?;
    super::permissions::validate_file(&file, true)?;
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

/// Publish a fully prepared temporary directory after refusing an observed
/// existing target. Unix anchors the rename; Linux additionally uses the
/// kernel's no-replace operation for the final step.
pub fn atomic_publish_directory(staging: tempfile::TempDir, target: &Path) -> Result<()> {
    atomic_publish_directory_with_hook(staging, target, || Ok(()))
}

#[cfg(unix)]
fn atomic_publish_directory_with_hook<F>(
    staging: tempfile::TempDir,
    target: &Path,
    hook: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    use std::{
        ffi::CString,
        os::{fd::AsRawFd, unix::ffi::OsStrExt},
    };

    super::files::reject_symlinks(target)?;
    let parent = target.parent().context("target directory has no parent")?;
    super::permissions::validate_directory(parent)?;
    let staging_parent = staging
        .path()
        .parent()
        .context("staging directory has no parent")?;
    ensure_same_directory(parent, staging_parent)?;
    let parent_file = open_directory_no_symlinks_unix(parent)?;
    let parent_identity = directory_identity_unix(&parent_file)?;
    let staging_name = CString::new(
        staging
            .path()
            .file_name()
            .context("staging directory has no file name")?
            .as_bytes(),
    )?;
    let target_name = CString::new(
        target
            .file_name()
            .context("target directory has no file name")?
            .as_bytes(),
    )?;
    ensure!(
        stat_at_unix(&parent_file, &target_name)?.is_none(),
        "refusing to replace existing directory {}",
        target.display()
    );

    hook()?;
    super::permissions::validate_directory(parent)?;
    let current_parent = open_directory_no_symlinks_unix(parent)?;
    ensure!(
        directory_identity_unix(&current_parent)? == parent_identity,
        "target directory changed during package publication"
    );

    #[cfg(any(target_os = "linux", target_os = "android"))]
    let result = unsafe {
        libc::renameat2(
            parent_file.as_raw_fd(),
            staging_name.as_ptr(),
            parent_file.as_raw_fd(),
            target_name.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    let result = unsafe {
        libc::renameat(
            parent_file.as_raw_fd(),
            staging_name.as_ptr(),
            parent_file.as_raw_fd(),
            target_name.as_ptr(),
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("failed to publish package at {}", target.display()));
    }
    parent_file.sync_all()?;
    // The old temporary path no longer exists, so its Drop cleanup is harmless.
    drop(staging);
    Ok(())
}

#[cfg(not(unix))]
fn atomic_publish_directory_with_hook<F>(
    staging: tempfile::TempDir,
    target: &Path,
    hook: F,
) -> Result<()>
where
    F: FnOnce() -> Result<()>,
{
    super::files::reject_symlinks(target)?;
    super::permissions::validate_directory(target.parent().context("target has no parent")?)?;
    ensure!(
        !target.exists(),
        "refusing to replace existing directory {}",
        target.display()
    );
    hook()?;
    super::files::reject_symlinks(target)?;
    super::permissions::validate_directory(target.parent().context("target has no parent")?)?;
    ensure!(
        !target.exists(),
        "refusing to replace existing directory {}",
        target.display()
    );
    fs::rename(staging.path(), target)
        .with_context(|| format!("failed to publish package at {}", target.display()))?;
    sync_directory(target.parent().context("target directory has no parent")?)?;
    drop(staging);
    Ok(())
}

#[cfg(unix)]
fn ensure_same_directory(left: &Path, right: &Path) -> Result<()> {
    ensure!(
        left.canonicalize()? == right.canonicalize()?,
        "staging directory must be in the publication directory"
    );
    Ok(())
}

#[cfg(unix)]
fn open_directory_no_symlinks_unix(path: &Path) -> Result<fs::File> {
    use std::{
        ffi::CString,
        os::{fd::FromRawFd, unix::ffi::OsStrExt},
        path::Component,
    };

    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let root = CString::new("/")?;
    let root_fd = unsafe {
        libc::open(
            root.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    if root_fd < 0 {
        return Err(std::io::Error::last_os_error()).context("failed to open filesystem root");
    }
    let mut directory = unsafe { fs::File::from_raw_fd(root_fd) };
    for component in absolute.components() {
        let Component::Normal(name) = component else {
            if matches!(component, Component::RootDir | Component::CurDir) {
                continue;
            }
            bail!("directory path contains an unsupported component");
        };
        let name = CString::new(name.as_bytes())?;
        let descriptor = unsafe {
            libc::openat(
                std::os::fd::AsRawFd::as_raw_fd(&directory),
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
        directory = unsafe { fs::File::from_raw_fd(descriptor) };
    }
    Ok(directory)
}

#[cfg(unix)]
fn directory_identity_unix(file: &fs::File) -> Result<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    let metadata = file.metadata()?;
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(unix)]
fn stat_at_unix(directory: &fs::File, name: &std::ffi::CStr) -> Result<Option<libc::stat>> {
    use std::os::fd::AsRawFd;
    let mut stat = std::mem::MaybeUninit::<libc::stat>::zeroed();
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(Some(unsafe { stat.assume_init() }));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(error).context("failed to inspect publication target")
    }
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
    super::permissions::validate_directory(parent)?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    super::files::reject_symlinks(path)?;

    let parent_file = open_directory_no_symlinks(parent)?;
    super::permissions::validate_file(&parent_file, false)?;
    if path.exists() {
        super::permissions::validate_path(path, false)?;
    }
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
    let original_digest = digest_at(&parent_file, &target_name)?;

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
    super::permissions::validate_file(&temporary_file, private)?;
    temporary_file.write_all(bytes)?;
    temporary_file.sync_all()?;
    hook()?;

    let current_parent = open_directory_no_symlinks(parent)?;
    super::permissions::validate_directory(parent)?;
    super::permissions::validate_file(&temporary_file, private)?;
    if path.exists() {
        super::permissions::validate_path(path, false)?;
    }
    if file_identity(&current_parent)? != parent_identity {
        bail!("target directory changed during atomic replacement");
    }
    let current = stat_at(&parent_file, &target_name)?;
    let current_digest = digest_at(&parent_file, &target_name)?;
    if !same_file_state(original.as_ref(), current.as_ref()) || original_digest != current_digest {
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

    fn digest_at(directory: &fs::File, name: &CStr) -> Result<Option<[u8; 32]>> {
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let descriptor = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            )
        };
        if descriptor < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::NotFound {
                return Ok(None);
            }
            return Err(error).context("failed to open atomic replacement target");
        }
        let mut file = unsafe { fs::File::from_raw_fd(descriptor) };
        ensure!(
            file.metadata()?.is_file(),
            "atomic replacement target is not a regular file"
        );
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
        Ok(Some(digest.finalize().into()))
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
    use sha2::{Digest, Sha256};
    use std::io::Read;

    super::files::reject_symlinks(path)?;
    let parent = path.parent().context("target path has no parent")?;
    super::permissions::validate_directory(parent)?;
    fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    super::files::reject_symlinks(path)?;
    super::permissions::validate_directory(parent)?;
    if path.exists() {
        super::permissions::validate_path(path, false)?;
    }
    let original_state = state(path)?;

    let mut temp = tempfile::Builder::new()
        .prefix(".lbc-write-")
        .tempfile_in(parent)?;
    if !private && let Ok(metadata) = fs::metadata(path) {
        temp.as_file().set_permissions(metadata.permissions())?;
    }
    super::permissions::validate_file(temp.as_file(), private)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    hook()?;
    super::files::reject_symlinks(path)?;
    super::permissions::validate_directory(parent)?;
    super::permissions::validate_file(temp.as_file(), private)?;
    if path.exists() {
        super::permissions::validate_path(path, false)?;
    }
    ensure!(
        state(path)? == original_state,
        "target changed during atomic replacement"
    );
    temp.persist(path)
        .with_context(|| format!("failed to replace {} atomically", path.display()))?;
    sync_directory(parent)?;
    return Ok(());

    fn state(path: &Path) -> Result<Option<(u64, Option<std::time::SystemTime>, [u8; 32])>> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        ensure!(metadata.is_file(), "target is not a regular file");
        let mut file = fs::File::open(path)?;
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 8192];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
        Ok(Some((
            metadata.len(),
            metadata.modified().ok(),
            digest.finalize().into(),
        )))
    }
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
    fn store_lock_rejects_a_fifo_without_blocking() -> Result<()> {
        use std::os::unix::ffi::OsStrExt;

        let root = tempfile::tempdir()?;
        let path = root.path().join("store.lock");
        let cpath = std::ffi::CString::new(path.as_os_str().as_bytes())?;
        assert_eq!(unsafe { libc::mkfifo(cpath.as_ptr(), 0o600) }, 0);
        assert!(lock_exclusive(&path).is_err());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn store_lock_rejects_symlinks_and_unsafe_permissions() -> Result<()> {
        use std::os::unix::fs::{PermissionsExt, symlink};

        let root = tempfile::tempdir()?;
        let target = root.path().join("target.lock");
        fs::write(&target, "")?;
        symlink(&target, root.path().join("linked.lock"))?;
        assert!(lock_exclusive(&root.path().join("linked.lock")).is_err());

        fs::set_permissions(&target, fs::Permissions::from_mode(0o644))?;
        assert!(lock_exclusive(&target).is_err());
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600))?;
        assert!(lock_exclusive(&target).is_ok());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn same_size_concurrent_edit_is_not_overwritten() -> Result<()> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("value");
        fs::write(&path, "original")?;

        let result = atomic_replace_with_hook(&path, b"replaced", false, || {
            fs::write(&path, "new-edit")?;
            Ok(())
        });

        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path)?, "new-edit");
        assert_eq!(fs::read_dir(root.path())?.count(), 1);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn permissions_changed_before_publication_preserve_original_and_cleanup() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir()?;
        let path = root.path().join("value");
        fs::write(&path, "original")?;
        let result = atomic_replace_with_hook(&path, b"replacement", true, || {
            fs::set_permissions(root.path(), fs::Permissions::from_mode(0o777))?;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path)?, "original");
        assert_eq!(fs::read_dir(root.path())?.count(), 1);
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))?;
        let result = atomic_replace_with_hook(&path, b"replacement", false, || {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o666))?;
            Ok(())
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

    #[cfg(unix)]
    #[test]
    fn directory_publication_rejects_parent_swap_and_existing_target() -> Result<()> {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        let parent = root.path().join("packages");
        let moved = root.path().join("packages-moved");
        fs::create_dir(&parent)?;
        let staging = tempfile::Builder::new()
            .prefix(".stage-")
            .tempdir_in(&parent)?;
        fs::write(staging.path().join("complete"), "snapshot")?;

        let result = atomic_publish_directory_with_hook(staging, &parent.join("demo"), || {
            fs::rename(&parent, &moved)?;
            symlink(outside.path(), &parent)?;
            Ok(())
        });
        assert!(result.is_err());
        assert!(!outside.path().join("demo").exists());
        assert!(!moved.join("demo").exists());

        fs::remove_file(&parent)?;
        fs::rename(&moved, &parent)?;
        fs::create_dir(parent.join("demo"))?;
        fs::write(parent.join("demo/existing"), "keep")?;
        let staging = tempfile::Builder::new()
            .prefix(".stage-")
            .tempdir_in(&parent)?;
        fs::write(staging.path().join("complete"), "new")?;
        assert!(atomic_publish_directory(staging, &parent.join("demo")).is_err());
        assert_eq!(fs::read_to_string(parent.join("demo/existing"))?, "keep");
        Ok(())
    }
}
