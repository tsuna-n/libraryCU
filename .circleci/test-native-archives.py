#!/usr/bin/env python3
"""Real archive-policy regressions, no native certificate/signature mocks."""
import hashlib
import io
from pathlib import Path
import stat
import tarfile
import tempfile
import zipfile

from native_release import packaged_binary

package = "lbc-0.5.0-x86_64-pc-windows-msvc"
files = ["lbc.exe", "install.ps1", "README.md", "CHANGELOG.md", "LICENSE"]
expected = hashlib.sha256(b"fixture").hexdigest()
passed = 0

with tempfile.TemporaryDirectory(prefix="lbc-native-archives-") as root:
    root = Path(root)
    for separator in ["/", "\\"]:
        path = root / "valid.zip"
        with zipfile.ZipFile(path, "w") as archive:
            for name in files:
                archive.writestr(f"{package}{separator}{name}", b"fixture")
        assert packaged_binary(path, package, True) == expected
        passed += 1
    for case in ["duplicate", "traversal", "backslash-traversal", "absolute", "symlink",
                 "root-symlink", "reparse", "special-file", "missing"]:
        path = root / f"{case}.zip"
        with zipfile.ZipFile(path, "w") as archive:
            for name in files[:-1] if case == "missing" else files:
                archive.writestr(f"{package}/{name}", b"fixture")
            if case == "duplicate":
                archive.writestr(f"{package}\\lbc.exe", b"fixture")
            elif case in ["traversal", "backslash-traversal"]:
                separator = "\\" if case.startswith("backslash") else "/"
                archive.writestr(f"{package}{separator}..{separator}escape", b"fixture")
            elif case == "absolute":
                archive.writestr(f"/{package}/escape", b"fixture")
            elif case == "symlink":
                entry = zipfile.ZipInfo(f"{package}/extra-link")
                entry.external_attr = (stat.S_IFLNK | 0o777) << 16
                archive.writestr(entry, b"lbc.exe")
            elif case == "root-symlink":
                entry = zipfile.ZipInfo(f"{package}/")
                entry.external_attr = (stat.S_IFLNK | 0o777) << 16
                archive.writestr(entry, b"")
            elif case == "reparse":
                entry = zipfile.ZipInfo(f"{package}/extra-link")
                entry.external_attr = 0x400
                archive.writestr(entry, b"lbc.exe")
            elif case == "special-file":
                entry = zipfile.ZipInfo(f"{package}/extra-fifo")
                entry.external_attr = (stat.S_IFIFO | 0o600) << 16
                archive.writestr(entry, b"fixture")
        try:
            packaged_binary(path, package, True)
        except ValueError:
            passed += 1
        else:
            raise AssertionError(f"unsafe ZIP accepted: {case}")
    for case in ["valid", "duplicate", "traversal", "backslash-traversal", "absolute",
                 "symlink", "hardlink", "missing"]:
        path = root / f"{case}.tar.gz"
        with tarfile.open(path, "w:gz") as archive:
            names = ["lbc", "install.sh", "README.md", "CHANGELOG.md", "LICENSE"]
            for name in names[:-1] if case == "missing" else names:
                entry = tarfile.TarInfo(f"{package}/{name}")
                entry.size = len(b"fixture")
                archive.addfile(entry, io.BytesIO(b"fixture"))
            if case not in ["valid", "missing"]:
                extra_name = {
                    "duplicate": f"{package}/lbc",
                    "backslash-traversal": f"{package}\\..\\escape",
                    "absolute": f"/{package}/escape",
                }.get(case, f"{package}/../escape")
                entry = tarfile.TarInfo(extra_name)
                entry.size = len(b"fixture")
                if case in ["symlink", "hardlink"]:
                    entry.name = f"{package}/extra-link"
                    entry.type = tarfile.SYMTYPE if case == "symlink" else tarfile.LNKTYPE
                    entry.linkname = "lbc"
                archive.addfile(entry, io.BytesIO(b"fixture"))
        try:
            value = packaged_binary(path, package, False)
        except ValueError:
            if case == "valid":
                raise
        else:
            assert case == "valid" and value == expected, f"unsafe tar accepted: {case}"
        passed += 1
assert passed == 19
print("Native archive policy: 19 passed (real ZIP/tar, including Windows separators)")
