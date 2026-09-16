//! Mutable stores are single-owner, not a multi-user collaboration mechanism.
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};

/// Validate the opened object, not a potentially stale pathname snapshot.
pub fn validate_file(file: &fs::File, private: bool) -> Result<()> {
    validate_file_inner(file, private, true)
}

fn validate_file_inner(file: &fs::File, private: bool, require_owner: bool) -> Result<()> {
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file() || metadata.is_dir(),
        "store object is not regular"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        validate_unix_mode(metadata.uid(), metadata.mode(), private)?;
        reject_acl(file)?;
    }
    #[cfg(windows)]
    windows::validate(file, private, require_owner)?;
    #[cfg(not(windows))]
    let _ = require_owner;
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        windows
    )))]
    anyhow::bail!("mutable-store ACL validation is unsupported on this platform");
    Ok(())
}

#[cfg(unix)]
fn validate_unix_mode(owner: u32, mode: u32, private: bool) -> Result<()> {
    // SAFETY: geteuid has no preconditions.
    ensure!(
        owner == unsafe { libc::geteuid() },
        "store object has a different owner"
    );
    ensure!(
        mode & if private { 0o077 } else { 0o022 } == 0,
        "store permissions grant unsafe group or other access"
    );
    Ok(())
}

/// Check an existing mutable-store object before reading or replacing it.
pub fn validate_path(path: &Path, private: bool) -> Result<()> {
    validate_path_inner(path, private, true)
}

fn validate_path_inner(path: &Path, private: bool, require_owner: bool) -> Result<()> {
    super::files::reject_symlinks(path)?;
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() || metadata.is_dir(),
        "store object is not regular"
    );
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        };
        options.custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
    }
    let file = options
        .open(path)
        .with_context(|| format!("cannot inspect store security: {}", path.display()))?;
    validate_file_inner(&file, private, require_owner)
        .with_context(|| format!("unsafe mutable store: {}", path.display()))
}

/// Refuse writable/foreign ancestors before creating store children. Root-owned
/// sticky temporary directories are the Unix exception, not store roots.
pub fn validate_directory(path: &Path) -> Result<()> {
    super::files::reject_symlinks(path)?;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    if absolute.exists() {
        validate_path(&absolute, false)?;
    }
    #[cfg(windows)]
    for ancestor in absolute.ancestors().skip(1) {
        if ancestor.exists() {
            validate_path_inner(ancestor, false, false)?;
        }
    }
    #[cfg(unix)]
    for ancestor in absolute.ancestors().skip(1) {
        let metadata = match fs::symlink_metadata(ancestor) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        use std::os::unix::fs::MetadataExt;
        ensure!(metadata.is_dir(), "store ancestor is not a directory");
        ensure!(
            metadata.uid() == 0 || metadata.uid() == unsafe { libc::geteuid() },
            "store ancestor has an untrusted owner: {}",
            ancestor.display()
        );
        if metadata.uid() == 0 && metadata.mode() & 0o1000 != 0 {
            continue;
        }
        ensure!(
            metadata.mode() & 0o022 == 0,
            "store ancestor is writable by another user: {}",
            ancestor.display()
        );
        reject_acl(&fs::File::open(ancestor)?)?;
    }
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn reject_acl(file: &fs::File) -> Result<()> {
    use std::os::fd::AsRawFd;
    for name in [c"system.posix_acl_access", c"system.posix_acl_default"] {
        // SAFETY: descriptor and NUL-terminated name are valid; zero length queries size.
        let size =
            unsafe { libc::fgetxattr(file.as_raw_fd(), name.as_ptr(), std::ptr::null_mut(), 0) };
        if size >= 0 {
            ensure!(
                size == 0,
                "extended or default ACL is not supported for mutable stores"
            );
        } else {
            let error = std::io::Error::last_os_error();
            // A filesystem without POSIX ACL support has no POSIX ACL grants.
            ensure!(
                matches!(error.raw_os_error(), Some(libc::ENODATA | libc::ENOTSUP)),
                "cannot validate store ACL: {error}"
            );
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn reject_acl(file: &fs::File) -> Result<()> {
    use std::{ffi::c_void, os::fd::AsRawFd};
    unsafe extern "C" {
        fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> *mut c_void;
        fn acl_get_entry(
            acl: *mut c_void,
            entry_id: libc::c_int,
            entry: *mut *mut c_void,
        ) -> libc::c_int;
        fn acl_free(acl: *mut c_void) -> libc::c_int;
    }
    // Apple sys/acl.h: ACL_TYPE_EXTENDED=0x100, ACL_FIRST_ENTRY=0.
    // SAFETY: the descriptor is live; returned allocation is freed exactly once.
    let acl = unsafe { acl_get_fd_np(file.as_raw_fd(), 0x100) };
    if acl.is_null() {
        let error = std::io::Error::last_os_error();
        ensure!(
            error.raw_os_error() == Some(libc::ENOENT),
            "cannot read store ACL: {error}"
        );
        return Ok(());
    }
    unsafe extern "C" {
        fn acl_get_tag_type(entry: *mut c_void, tag: *mut libc::c_int) -> libc::c_int;
    }
    let mut entry = std::ptr::null_mut();
    let mut index = 0;
    let validation = (|| {
        loop {
            let result = unsafe { acl_get_entry(acl, index, &mut entry) };
            if result == -1 {
                ensure!(
                    std::io::Error::last_os_error().raw_os_error() == Some(libc::EINVAL),
                    "cannot enumerate store ACL"
                );
                break;
            }
            ensure!(result == 0, "cannot enumerate store ACL");
            let mut tag = 0;
            ensure!(
                unsafe { acl_get_tag_type(entry, &mut tag) } == 0 && tag == 2,
                "extended ACL grants are not supported for mutable stores"
            );
            index = -1; // ACL_NEXT_ENTRY; harmless deny entries may remain.
        }
        Ok(())
    })();
    unsafe { acl_free(acl) };
    validation
}

#[cfg(all(
    unix,
    not(any(target_os = "linux", target_os = "android", target_os = "macos"))
))]
fn reject_acl(_: &fs::File) -> Result<()> {
    anyhow::bail!("mutable-store ACL validation is unsupported on this platform")
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{ffi::c_void, os::windows::io::AsRawHandle, ptr};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, LocalFree},
        Security::{
            Authorization::{GetSecurityInfo, SE_FILE_OBJECT},
            *,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };

    struct Descriptor(PSECURITY_DESCRIPTOR);
    impl Drop for Descriptor {
        fn drop(&mut self) {
            unsafe { LocalFree(self.0) };
        }
    }
    struct Token(windows_sys::Win32::Foundation::HANDLE);
    impl Drop for Token {
        fn drop(&mut self) {
            unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) fn validate(file: &fs::File, private: bool, require_owner: bool) -> Result<()> {
        use std::os::windows::fs::MetadataExt;
        ensure!(
            file.metadata()?.file_attributes() & 0x400 == 0,
            "store object is a reparse point"
        );
        // SAFETY: Win32 APIs receive live handles and valid out-parameters.
        unsafe {
            let mut owner = ptr::null_mut();
            let mut acl = ptr::null_mut();
            let mut descriptor = ptr::null_mut();
            let result = GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                ptr::null_mut(),
                &mut acl,
                ptr::null_mut(),
                &mut descriptor,
            );
            ensure!(
                result == 0,
                "cannot inspect Windows store security: {result}"
            );
            let _descriptor = Descriptor(descriptor);
            ensure!(
                !owner.is_null() && IsValidSid(owner) != 0,
                "invalid store owner SID"
            );
            ensure!(
                !acl.is_null() && IsValidAcl(acl) != 0,
                "missing or invalid store DACL"
            );

            let mut token = ptr::null_mut();
            ensure!(
                OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) != 0,
                "cannot open process token"
            );
            let token = Token(token);
            let mut size = 0;
            GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &mut size);
            ensure!(
                size >= std::mem::size_of::<TOKEN_USER>() as u32,
                "invalid token user size"
            );
            // usize storage ensures TOKEN_USER pointer alignment.
            let mut buffer = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
            ensure!(
                GetTokenInformation(
                    token.0,
                    TokenUser,
                    buffer.as_mut_ptr().cast(),
                    size,
                    &mut size
                ) != 0,
                "cannot read token user"
            );
            let user = (*(buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid;
            ensure!(
                EqualSid(owner, user) != 0
                    || (!require_owner
                        && (IsWellKnownSid(owner, WinLocalSystemSid) != 0
                            || IsWellKnownSid(owner, WinBuiltinAdministratorsSid) != 0)),
                "store object has a different owner"
            );

            for index in 0..(*acl).AceCount as u32 {
                let mut ace: *mut c_void = ptr::null_mut();
                ensure!(
                    GetAce(acl, index, &mut ace) != 0,
                    "cannot inspect store ACE"
                );
                let header = &*(ace.cast::<ACE_HEADER>());
                // Denies cannot grant access. Unknown/callback/object ACEs fail closed.
                if header.AceType == 1 {
                    continue;
                }
                ensure!(
                    header.AceType == 0
                        && header.AceSize as usize >= std::mem::size_of::<ACCESS_ALLOWED_ACE>(),
                    "unsupported store ACE type"
                );
                let allowed = &*(ace.cast::<ACCESS_ALLOWED_ACE>());
                let sid = ptr::addr_of!(allowed.SidStart) as PSID;
                ensure!(IsValidSid(sid) != 0, "invalid store ACE SID");
                let trusted = EqualSid(sid, user) != 0
                    || IsWellKnownSid(sid, WinLocalSystemSid) != 0
                    || IsWellKnownSid(sid, WinBuiltinAdministratorsSid) != 0
                    || (header.AceFlags as u32 & INHERIT_ONLY_ACE != 0
                        && IsWellKnownSid(sid, WinCreatorOwnerSid) != 0);
                // Includes write/add/delete-child, attributes, DELETE, WRITE_DAC,
                // WRITE_OWNER and generic write/all. Do not let denies offset grants.
                let unsafe_mask = if private {
                    u32::MAX
                } else if require_owner {
                    !0xa012_00a9 // Only known read/execute/synchronize rights.
                } else {
                    0x500d_0140 // Ancestor create-only grants cannot replace existing children.
                };
                ensure!(
                    trusted || allowed.Mask & unsafe_mask == 0,
                    "store DACL grants unsafe access to another principal"
                );
            }
        }
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn owner_and_mode_policy_boundaries() {
        let owner = unsafe { libc::geteuid() };
        assert!(validate_unix_mode(owner.wrapping_add(1), 0o600, true).is_err());
        for mode in [0o400, 0o600, 0o700] {
            assert!(validate_unix_mode(owner, mode, true).is_ok());
        }
        for mode in [0o604, 0o640, 0o660, 0o777] {
            assert!(validate_unix_mode(owner, mode, true).is_err());
        }
        for mode in [0o644, 0o755] {
            assert!(validate_unix_mode(owner, mode, false).is_ok());
        }
        for mode in [0o622, 0o664, 0o775, 0o1777] {
            assert!(validate_unix_mode(owner, mode, false).is_err());
        }
    }
}
