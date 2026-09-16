# Mutable-store ownership and ACL policy

libraryCube v0.5 mutable stores are **single-owner**. Concurrent processes from
that owner can use advisory locks; this is not multi-user collaboration or RBAC.
Unsafe pre-existing stores are refused, not silently chmod/chowned or repaired.
Knowledge remains untrusted input even when the filesystem passes these checks.

## Supported platform policy

| Platform | Owner and permissions | ACL handling |
|---|---|---|
| Linux | Effective UID owns store objects; no group/other write on store directories or mutable public files; no group/other access on locks, new private data, or recovery keys | Reject any extended POSIX access ACL, even with a zero mask, and any default ACL. Absent ACLs and filesystems reporting POSIX ACLs unsupported are accepted; other inspection failures fail closed |
| macOS | Same effective-UID and mode policy | Inspect the opened descriptor's extended ACL; reject allow entries, including inherited/inherit-only grants. Deny-only ACLs are harmless and retained |
| Windows | Store object owner must match the process token user; store ancestors may also be SYSTEM/Administrators-owned | Read the opened handle's owner and DACL. Only the current user, SYSTEM, and Administrators may have mutation/private-data rights; inherit-only CREATOR OWNER entries are allowed. Other principals may have documented read/execute rights on public mutable objects. NULL/invalid DACLs, unknown ACE types, and reparse points/junctions fail closed |

Unix ancestors must be owned by the effective user or root and must not grant
group/other writes or ACL grants. Root-owned sticky temporary directories are
allowed as ancestors, never as store roots. Windows ancestor create-only grants
are allowed, but grants that permit replacing children or changing access control
are refused. Inherited grants are checked before writing sensitive temporary bytes.
Privileged administrators/root remain trusted and are not isolated by this policy.

## Where enforcement applies

`src/security/permissions.rs` supplies descriptor/handle checks used by store
locks, atomic file replacement, package directory publication, knowledge creation,
configuration reads, persistent-history reads, and recovery-key creation/reads.
Locks and atomic replacement recheck security before publication. Config files
may remain owner-controlled `0644`; private history/key reads require private
access. Atomic private replacement can tighten an owner-controlled public file,
but it refuses an existing file writable by another principal. Read-only project
source/knowledge inspection is not treated as trusted mutable storage.

## Remediation and limits

Use a separate owner-controlled store per account. Move data out of writable
shared directories, verify its contents, remove unwanted ACL grants/default ACLs,
and restrict sensitive files (`0600`) before retrying. On Windows use a private
NTFS directory with current-user/SYSTEM/Administrators access. LBC will not modify
ACLs or ownership for you. A filesystem whose security cannot be inspected is
not suitable for sensitive mutable stores.

These checks are fail-closed observations, not a sandbox or guarantee against
every non-cooperating writer. Unix publication is directory-descriptor anchored;
Windows portable publication still has path-based race limits. Atomic replacement
does not preserve extended ACLs (grants are rejected); harmless macOS denies on an
old file are not copied to its replacement. Windows temporary files inherit the
validated parent DACL. Network filesystem ACL models beyond the native APIs are
not certified. Unknown platforms refuse mutable-store access.

## Regression evidence

`tests/filesystem.rs` covers safe lock/private-write round trips, unsafe shared
roots/ancestors, preservation of unsafe existing files, private-read modes, masked
Linux access ACLs and default ACL inheritance, macOS allow/deny/inheritance,
Windows foreign-principal grants/NULL DACLs/junctions, and the existing symlink,
FIFO, size, and containment boundaries. Unit tests cover owner/mode boundaries,
permission changes during publication, failure cleanup, concurrent edits, and
parent swaps. Platform-specific tests must execute on that platform; a Linux
pass does not prove the macOS/Windows rows passed.

Native API references: [Microsoft GetSecurityInfo](https://learn.microsoft.com/en-us/windows/win32/api/aclapi/nf-aclapi-getsecurityinfo),
[Apple ACL definitions](https://github.com/apple-oss-distributions/Libc/blob/main/include/sys/acl.h),
[Apple ACL enumeration](https://github.com/apple-oss-distributions/Libc/blob/main/posix1e/acl_entry.c),
and [Linux ACL xattr format](https://github.com/torvalds/linux/blob/master/include/uapi/linux/posix_acl_xattr.h).
