#!/usr/bin/env python3
"""Synthetic archive/native-record data for tests ONLY; no native trust evidence."""

import io
import json
import os
from pathlib import Path
import sys
import tarfile
import zipfile

from native_release import digest, packaged_binary


def create(dist: Path, version: str, source: str, workflow: str) -> None:
    dist.mkdir(parents=True, exist_ok=True)
    for suffix in ["x86_64-unknown-linux-gnu", "universal-apple-darwin", "x86_64-pc-windows-msvc"]:
        windows = suffix.endswith("msvc")
        package = f"lbc-{version}-{suffix}"
        names = ["lbc.exe" if windows else "lbc", "install.ps1" if windows else "install.sh", "README.md", "CHANGELOG.md", "LICENSE"]
        content = {name: (f"#!/usr/bin/env bash\nprintf 'lbc {version}\\n'\n".encode()
                          if name == "lbc" else b"synthetic release fixture\n") for name in names}
        archive = f"{package}.zip" if windows else f"{package}.tar.gz"
        if windows:
            with zipfile.ZipFile(dist / archive, "w") as output:
                for name, data in content.items():
                    output.writestr(f"{package}/{name}", data)
        else:
            with tarfile.open(dist / archive, "w:gz") as output:
                for name, data in content.items():
                    entry = tarfile.TarInfo(f"{package}/{name}")
                    entry.size = len(data); entry.mode = 0o755 if name in ["lbc", "install.sh"] else 0o644
                    output.addfile(entry, io.BytesIO(data))
        assets = [archive]
        if suffix == "universal-apple-darwin":
            dmg = f"{package}.dmg"
            (dist / dmg).write_bytes(b"SIMULATED DMG: NOT a signed/notarized image\n")
            assets.append(dmg)
        for name in assets:
            (dist / f"{name}.sha256").write_text(f"{digest(dist / name)}  {name}\n", encoding="utf-8")
        if suffix == "x86_64-unknown-linux-gnu":
            continue
        platform = "windows" if windows else "macos"
        pin = os.environ.get("LBC_WINDOWS_CERT_THUMBPRINT" if windows else "LBC_MACOS_IDENTITY_SHA1", "A" * 40)
        signer = {"certificateSha1": pin}
        checks = {"signature": True, "timestamp": True, "releaseCli": True, "packagedBinary": True}
        if not windows:
            signer["teamId"] = os.environ.get("LBC_MACOS_TEAM_ID", "FIXTURE123")
            checks.update(hardenedRuntime=True, gatekeeper=True, notarization={
                "id": "11111111-1111-4111-8111-111111111111", "status": "Accepted", "stapled": True, "validated": True})
        record = {"schemaVersion": 1, "kind": "librarycube/native-verification/v1", "platform": platform,
                  "mode": "production", "repository": "tsuna-n/libraryCU", "sourceSha": source,
                  "version": version, "tag": f"v{version}", "workflowId": workflow, "jobName": f"sign_{platform}_release",
                  "signer": signer, "verification": checks,
                  "binary": {"path": f"{package}/{'lbc.exe' if windows else 'lbc'}",
                             "sha256": packaged_binary(dist / archive, package, windows)},
                  "archives": [{"name": name, "sha256": digest(dist / name)} for name in assets]}
        (dist / f"lbc-{version}.{platform}-signing.json").write_text(json.dumps(record), encoding="utf-8")
    (dist / f"lbc-{version}.cdx.json").write_text(json.dumps({"bomFormat": "CycloneDX", "specVersion": "1.3", "version": 1,
        "components": [{"name": "fixture", "version": "1.0.0", "licenses": [{"license": {"id": "MIT"}}],
                        "hashes": [{"alg": "SHA-256", "content": "0" * 64}]}]}), encoding="utf-8")


if __name__ == "__main__":
    create(Path(sys.argv[1]), *sys.argv[2:5])
