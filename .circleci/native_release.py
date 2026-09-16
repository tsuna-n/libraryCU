#!/usr/bin/env python3
"""Validate digest-bound native-job records, not hosted/control-plane attestation.

Actual platform signatures/tickets are verified by the tag-only native jobs and
must also be verified independently on Windows/macOS after download. A JSON
record alone cannot establish certificate trust or SLSA Build Level 2.
"""

import hashlib
import os
from pathlib import Path
import re
import stat
import sys
import tarfile
import zipfile

from release_json import load


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(path: Path) -> str:
    require(path.is_file() and not path.is_symlink(), f"unsafe native asset: {path.name}")
    with path.open("rb") as handle:
        return stream_digest(handle)


def stream_digest(handle) -> str:
    value = hashlib.sha256()
    for chunk in iter(lambda: handle.read(1024 * 1024), b""):
        value.update(chunk)
    return value.hexdigest()


def packaged_binary(path: Path, package: str, windows: bool) -> str:
    binary = "lbc.exe" if windows else "lbc"
    expected = {f"{package}/{name}" for name in
                [binary, "install.ps1" if windows else "install.sh", "README.md", "CHANGELOG.md", "LICENSE"]}
    seen: set[str] = set()
    result = ""
    if windows:
        with zipfile.ZipFile(path) as archive:
            for item in archive.infolist():
                name = item.filename.rstrip("/")
                require(name not in seen, "duplicated ZIP member")
                seen.add(name)
                if item.is_dir():
                    require(name == package, "unexpected ZIP directory")
                    continue
                require(item.filename in expected and not stat.S_ISLNK(item.external_attr >> 16),
                        "unsafe/unexpected ZIP member")
                require(0 < item.file_size <= 128 * 1024 * 1024, "invalid ZIP member size")
                if name == f"{package}/{binary}":
                    with archive.open(item) as handle:
                        result = stream_digest(handle)
    else:
        with tarfile.open(path, "r:gz") as archive:
            for item in archive:
                name = item.name.rstrip("/")
                require(name not in seen, "duplicated tar member")
                seen.add(name)
                if item.isdir():
                    require(name == package, "unexpected tar directory")
                    continue
                require(name in expected and item.isreg(), "unsafe/unexpected tar member")
                require(0 < item.size <= 128 * 1024 * 1024, "invalid tar member size")
                if name == f"{package}/{binary}":
                    with archive.extractfile(item) as handle:
                        result = stream_digest(handle)
    require(expected <= seen and bool(result), "native archive has an incomplete package")
    return result


def validate(dist: Path, version: str, source_sha: str, workflow_id: str) -> None:
    require(bool(re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", source_sha)), "invalid native source SHA")
    for platform in ["windows", "macos"]:
        record = load(str(dist / f"lbc-{version}.{platform}-signing.json"))
        require(isinstance(record, dict), "native record must be an object")
        require(type(record.get("schemaVersion")) is int and record.get("schemaVersion") == 1
                and record.get("kind") == "librarycube/native-verification/v1",
                "unknown native verification record")
        for field, expected in {"platform": platform, "mode": "production", "repository": "tsuna-n/libraryCU",
                                "sourceSha": source_sha, "version": version, "tag": f"v{version}",
                                "workflowId": workflow_id, "jobName": f"sign_{platform}_release"}.items():
            require(record.get(field) == expected and bool(expected), f"native {field} mismatch")
        pin_name = "LBC_WINDOWS_CERT_THUMBPRINT" if platform == "windows" else "LBC_MACOS_IDENTITY_SHA1"
        pin = os.environ.get(pin_name, "")
        require(bool(re.fullmatch(r"[0-9A-F]{40}", pin)), f"EXTERNAL CREDENTIAL REQUIRED: {pin_name}")
        require(record.get("signer", {}).get("certificateSha1") == pin, "native signer pin mismatch")
        checks = record.get("verification", {})
        for check in ["signature", "timestamp", "releaseCli", "packagedBinary"]:
            require(checks.get(check) is True, f"missing native verification: {check}")
        suffix = "x86_64-pc-windows-msvc" if platform == "windows" else "universal-apple-darwin"
        package = f"lbc-{version}-{suffix}"
        archive = f"{package}.zip" if platform == "windows" else f"{package}.tar.gz"
        names = [archive]
        if platform == "macos":
            team = os.environ.get("LBC_MACOS_TEAM_ID", "")
            require(bool(re.fullmatch(r"[A-Z0-9]{10}", team)) and record.get("signer", {}).get("teamId") == team,
                    "EXTERNAL CREDENTIAL REQUIRED: macOS Team ID pin")
            require(checks.get("hardenedRuntime") is True and checks.get("gatekeeper") is True,
                    "missing runtime/Gatekeeper checks")
            notary = checks.get("notarization", {})
            require(notary.get("status") == "Accepted" and notary.get("stapled") is True
                    and notary.get("validated") is True
                    and bool(re.fullmatch(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}", notary.get("id", ""))),
                    "missing accepted/stapled/validated notarization")
            names.append(f"{package}.dmg")
        expected_archives = [{"name": name, "sha256": digest(dist / name)} for name in names]
        require(record.get("archives") == expected_archives, "native final archive digests mismatch")
        binary = "lbc.exe" if platform == "windows" else "lbc"
        require(record.get("binary") == {"path": f"{package}/{binary}",
                                         "sha256": packaged_binary(dist / archive, package, platform == "windows")},
                "packaged native binary digest mismatch")


if __name__ == "__main__":
    try:
        if sys.argv[1] == "package-hash":
            print(packaged_binary(Path(sys.argv[2]), sys.argv[3], sys.argv[4] == "windows"))
        else:
            validate(Path(sys.argv[1]), *sys.argv[2:5])
    except (OSError, ValueError, TypeError, AttributeError, tarfile.TarError, zipfile.BadZipFile) as error:
        sys.exit(f"Native release gate failed: {error}")
