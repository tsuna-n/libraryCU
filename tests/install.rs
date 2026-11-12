#[cfg(unix)]
mod unix {
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        io::Write,
        path::Path,
        process::{Command, Output, Stdio},
    };

    fn package() -> String {
        let target = if cfg!(target_os = "macos") {
            "universal-apple-darwin"
        } else {
            "x86_64-unknown-linux-gnu"
        };
        format!("lbc-{}-{target}", env!("CARGO_PKG_VERSION"))
    }

    fn member(tar: &mut Vec<u8>, name: &str, data: &[u8], kind: u8) {
        let mut header = [0u8; 512];
        assert!(name.len() < 100);
        header[..name.len()].copy_from_slice(name.as_bytes());
        for (range, value) in [
            (100..108, "0000755\0"),
            (108..116, "0000000\0"),
            (116..124, "0000000\0"),
        ] {
            header[range].copy_from_slice(value.as_bytes());
        }
        header[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes());
        header[136..148].copy_from_slice(b"00000000000\0");
        header[148..156].fill(b' ');
        header[156] = kind;
        header[257..263].copy_from_slice(b"ustar\0");
        header[263..265].copy_from_slice(b"00");
        let checksum: u32 = header.iter().map(|&b| u32::from(b)).sum();
        header[148..156].copy_from_slice(format!("{checksum:06o}\0 ").as_bytes());
        tar.extend(header);
        tar.extend(data);
        tar.resize(tar.len().next_multiple_of(512), 0);
    }

    fn archive(root: &Path, members: &[(&str, &[u8], u8)]) -> (std::path::PathBuf, String) {
        let archive = root.join("download.tar.gz");
        let mut tar = Vec::new();
        for (name, data, kind) in members {
            member(&mut tar, name, data, *kind);
        }
        tar.resize(tar.len() + 1024, 0);
        let file = fs::File::create(&archive).unwrap();
        let mut gzip = Command::new("gzip")
            .arg("-c")
            .stdin(Stdio::piped())
            .stdout(file)
            .spawn()
            .unwrap();
        gzip.stdin.take().unwrap().write_all(&tar).unwrap();
        assert!(gzip.wait().unwrap().success());
        let digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
        (archive, digest)
    }

    fn install(archive: &Path, digest: &str, prefix: &Path, shell: &str) -> Output {
        Command::new("bash")
            .arg("install.sh")
            .args(["--archive"])
            .arg(archive)
            .args(["--sha256", digest, "--prefix"])
            .arg(prefix)
            .env("SHELL", shell)
            .output()
            .unwrap()
    }

    #[test]
    fn checksum_and_argument_failures_do_not_mutate_the_prefix() {
        let root = tempfile::tempdir().unwrap();
        let archive = root.path().join("archive");
        fs::write(&archive, "not an archive").unwrap();
        let prefix = root.path().join("missing-prefix");
        let result = install(&archive, &"0".repeat(64), &prefix, "/bin/bash");
        assert!(!result.status.success());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains("SHA-256 mismatch"),
            "{result:?}"
        );
        assert!(!prefix.exists());
        for args in [vec!["--archive", "file"], vec!["--sha256", "bad"]] {
            let result = Command::new("bash")
                .arg("install.sh")
                .args(args)
                .output()
                .unwrap();
            assert!(!result.status.success());
            assert!(
                String::from_utf8_lossy(&result.stderr).contains("--archive and --sha256"),
                "{result:?}"
            );
        }
    }

    #[test]
    fn verified_archive_installs_real_binary_and_preserves_user_files_on_uninstall() {
        let root = tempfile::tempdir().unwrap();
        let package = package();
        let binary = fs::read(env!("CARGO_BIN_EXE_lbc")).unwrap();
        let names: Vec<_> = ["lbc", "install.sh", "README.md", "CHANGELOG.md", "LICENSE"]
            .iter()
            .map(|name| format!("{package}/{name}"))
            .collect();
        // The packaged installer is untrusted data and must never execute.
        let data: [&[u8]; 5] = [&binary, b"exit 99", b"readme", b"changes", b"license"];
        let members: Vec<_> = names
            .iter()
            .zip(data)
            .map(|(name, data)| (name.as_str(), data, b'0'))
            .collect();
        let (archive, digest) = archive(root.path(), &members);
        let prefix = root
            .path()
            .join("prefix 'quotes' $HOME $(touch INJECTED) `touch INJECTED2`");
        let stubs = root.path().join("no-rust");
        fs::create_dir(&stubs).unwrap();
        use std::os::unix::fs::PermissionsExt;
        for name in ["cargo", "rustc", "rustup"] {
            let stub = stubs.join(name);
            fs::write(&stub, "#!/bin/sh\ntouch \"$RUST_CALLED\"\nexit 99\n").unwrap();
            fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
        }
        for shell in ["/bin/bash", "/bin/zsh", "/usr/bin/fish"] {
            let result = Command::new("bash")
                .arg("install.sh")
                .arg("--archive")
                .arg(&archive)
                .args(["--sha256", &digest, "--prefix"])
                .arg(&prefix)
                .env("SHELL", shell)
                .env(
                    "PATH",
                    format!("{}:{}", stubs.display(), std::env::var("PATH").unwrap()),
                )
                .env("RUST_CALLED", root.path().join("rust-called"))
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
            let output = String::from_utf8_lossy(&result.stdout);
            assert!(output.contains("lbc setup"), "{output}");
            assert!(
                output.contains("lbc learn")
                    && output.contains("lbc search")
                    && output.contains("lbc explain")
            );
            assert!(output.contains(if shell.ends_with("fish") {
                "fish_add_path -- "
            } else {
                "export PATH="
            }));
            assert!(!root.path().join("rust-called").exists());
            if !shell.ends_with("fish") {
                let line = output
                    .lines()
                    .find(|line| line.trim_start().starts_with("export PATH="))
                    .unwrap();
                let result = Command::new("bash")
                    .args(["-c", &format!("{line}\nprintf '%s' \"$PATH\"")])
                    .current_dir(root.path())
                    .output()
                    .unwrap();
                assert!(result.status.success());
                assert!(
                    String::from_utf8_lossy(&result.stdout)
                        .starts_with(&format!("{}:", prefix.display()))
                );
                assert!(!root.path().join("INJECTED").exists());
                assert!(!root.path().join("INJECTED2").exists());
            }
        }
        let version = Command::new(prefix.join("lbc"))
            .arg("--version")
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&version.stdout).trim(),
            format!("lbc {}", env!("CARGO_PKG_VERSION"))
        );
        fs::write(prefix.join("knowledge.md"), "mine").unwrap();
        for _ in 0..2 {
            let result = Command::new("bash")
                .arg("install.sh")
                .arg("--prefix")
                .arg(&prefix)
                .arg("--uninstall")
                .output()
                .unwrap();
            assert!(result.status.success(), "{result:?}");
            assert!(!prefix.join("lbc").exists());
            assert_eq!(
                fs::read_to_string(prefix.join("knowledge.md")).unwrap(),
                "mine"
            );
        }
    }

    #[test]
    fn unsafe_or_invalid_packages_leave_an_existing_install_untouched() {
        let root = tempfile::tempdir().unwrap();
        let prefix = root.path().join("prefix");
        fs::create_dir(&prefix).unwrap();
        fs::write(prefix.join("lbc"), "existing binary").unwrap();
        let package = package();
        let names: Vec<_> = ["lbc", "install.sh", "README.md", "CHANGELOG.md", "LICENSE"]
            .iter()
            .map(|name| format!("{package}/{name}"))
            .collect();
        let base: Vec<_> = names
            .iter()
            .map(|name| (name.as_str(), b"invalid executable".as_slice(), b'0'))
            .collect();
        for case in [
            "invalid-binary",
            "missing",
            "duplicate",
            "traversal",
            "absolute",
            "symlink",
            "hardlink",
            "fifo",
            "unexpected",
            "empty",
            "empty-archive",
            "directory",
            "platform",
            "oversized",
            "corrupt",
            "corrupt-executable",
        ] {
            let mut members = base.clone();
            let oversized;
            match case {
                "missing" => {
                    members.pop();
                }
                "duplicate" => members.push(base[0]),
                "traversal" => members.push(("../outside", b"escape", b'0')),
                "absolute" => members.push(("/tmp/lbc-installer-escape", b"escape", b'0')),
                "symlink" => members[0].2 = b'2',
                "hardlink" => members[0].2 = b'1',
                "fifo" => members[0].2 = b'6',
                "unexpected" => members.push(("unexpected", b"extra", b'0')),
                "empty" => members[0].1 = b"",
                "empty-archive" => members.clear(),
                "directory" => members[0].2 = b'5',
                "platform" => members[0].0 = "lbc-0.5.0-unsupported-platform/lbc",
                "oversized" => {
                    oversized = vec![0; 128 * 1024 * 1024 + 1];
                    members[0].1 = &oversized;
                }
                "corrupt-executable" => members[0].1 = b"\x7fELFcorrupt executable",
                _ => {}
            }
            let (archive, mut digest) = archive(root.path(), &members);
            if case == "corrupt" {
                fs::write(&archive, "corrupt archive").unwrap();
                digest = format!("{:x}", Sha256::digest(fs::read(&archive).unwrap()));
            }
            let result = install(&archive, &digest, &prefix, "/bin/bash");
            assert!(!result.status.success(), "accepted {case}: {result:?}");
            if case == "empty-archive" {
                assert!(
                    String::from_utf8_lossy(&result.stderr).contains("Empty release archive"),
                    "{result:?}"
                );
            }
            assert_eq!(
                fs::read_to_string(prefix.join("lbc")).unwrap(),
                "existing binary",
                "{case}"
            );
            assert_eq!(fs::read_dir(&prefix).unwrap().count(), 1);
        }
    }
}

#[cfg(windows)]
#[test]
fn windows_verified_zip_install_path_and_uninstall_preserve_user_data() {
    use std::{fs, process::Command};
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("test-install.ps1");
    fs::write(&script, r#"
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
$package = 'lbc-' + $env:TEST_VERSION + '-x86_64-pc-windows-msvc'
$archive = Join-Path $env:TEST_ROOT 'download.zip'
$prefix = Join-Path $env:TEST_ROOT "prefix with spaces and 'quotes'"
$binary = Join-Path $prefix 'lbc.exe'
$oldPath = [Environment]::GetEnvironmentVariable('Path', 'User')
$powerShell = (Get-Process -Id $PID).Path
function New-TestArchive([string]$Case) {
    if (Test-Path -LiteralPath $archive) { Remove-Item -LiteralPath $archive }
    $zip = [IO.Compression.ZipFile]::Open($archive, [IO.Compression.ZipArchiveMode]::Create)
    try {
        $names = @('lbc.exe', 'install.ps1', 'README.md', 'CHANGELOG.md', 'LICENSE')
        if ($Case -eq 'missing') { $names = $names[0..3] }
        if ($Case -eq 'duplicate') { $names += 'lbc.exe' }
        if ($Case -eq 'traversal') { $names += '../escape' }
        if ($Case -eq 'absolute') { $names += '/escape' }
        if ($Case -eq 'unexpected') { $names += 'extra' }
        if ($Case -eq 'empty-archive') { $names = @() }
        foreach ($name in $names) {
            $entryName = "$package/$name"
            if ($Case -eq 'backslash') { $entryName = $entryName.Replace('/', '\') }
            $entry = $zip.CreateEntry($entryName, [IO.Compression.CompressionLevel]::Fastest)
            if ($Case -eq 'symlink' -and $name -eq 'lbc.exe') { $entry.ExternalAttributes = -1577123840 }
            $stream = $entry.Open()
            try {
                if ($name -eq 'lbc.exe' -and $Case -in @('valid', 'backslash')) {
                    $inputStream = [IO.File]::OpenRead($env:TEST_BINARY)
                    try { $inputStream.CopyTo($stream) } finally { $inputStream.Dispose() }
                } elseif ($Case -eq 'oversized' -and $name -eq 'lbc.exe') {
                    $buffer = New-Object byte[] (1MB)
                    for ($i = 0; $i -lt 129; $i++) { $stream.Write($buffer, 0, $buffer.Length) }
                } elseif ($Case -ne 'empty' -or $name -ne 'lbc.exe') {
                    $bytes = [Text.Encoding]::UTF8.GetBytes('invalid executable; packaged installer must not run')
                    $stream.Write($bytes, 0, $bytes.Length)
                }
            } finally { $stream.Dispose() }
        }
    } finally { $zip.Dispose() }
    return (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
}
function Invoke-Installer([string]$Digest, [bool]$NoPath) {
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $env:TEST_INSTALLER,
        '-Archive', $archive, '-Sha256', $Digest, '-Prefix', $prefix)
    if ($NoPath) { $arguments += '-NoPathUpdate' }
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = & $powerShell @arguments 2>&1 | Out-String
        return @{ Code = $LASTEXITCODE; Output = $output }
    } finally { $ErrorActionPreference = $previousPreference }
}
try {
    $digest = New-TestArchive 'invalid'
    $result = Invoke-Installer ('0' * 64) $true
    if ($result.Code -eq 0 -or $result.Output -notmatch 'SHA-256 mismatch' -or (Test-Path -LiteralPath $prefix)) {
        throw "Checksum failure mutated the prefix or lacked its diagnostic: $($result.Output)"
    }
    New-Item -ItemType Directory -Path $prefix | Out-Null
    [IO.File]::WriteAllText($binary, 'existing binary')
    foreach ($case in @('invalid', 'missing', 'duplicate', 'traversal', 'absolute', 'unexpected', 'symlink', 'empty', 'empty-archive', 'oversized')) {
        $digest = New-TestArchive $case
        $result = Invoke-Installer $digest $true
        if ($result.Code -eq 0 -or [IO.File]::ReadAllText($binary) -cne 'existing binary') {
            throw "Unsafe package accepted or previous binary changed ($case): $($result.Output)"
        }
    }
    foreach ($case in @('valid', 'backslash')) {
        $digest = New-TestArchive $case
        $result = Invoke-Installer $digest $true
        if ($result.Code -ne 0 -or $result.Output -notmatch 'lbc setup' -or $result.Output -notmatch 'lbc explain build.log') {
            throw "Verified ZIP failed: $($result.Output)"
        }
    }
    $version = (& $binary --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $version -cne "lbc $env:TEST_VERSION") { throw 'Installed version mismatch' }
    $result = Invoke-Installer $digest $false
    if ($result.Code -ne 0) { throw "Persistent PATH install failed: $($result.Output)" }
    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (($path -split ';') -inotcontains $prefix) { throw 'Persistent PATH entry missing' }
    $note = Join-Path $prefix 'knowledge.md'
    [IO.File]::WriteAllText($note, 'mine')
    for ($i = 0; $i -lt 2; $i++) {
        & $powerShell -NoProfile -ExecutionPolicy Bypass -File $env:TEST_INSTALLER -Prefix $prefix -Uninstall
        if ($LASTEXITCODE -ne 0 -or (Test-Path -LiteralPath $binary) -or [IO.File]::ReadAllText($note) -cne 'mine') {
            throw 'Uninstall removed user data or failed'
        }
        if (([Environment]::GetEnvironmentVariable('Path', 'User') -split ';') -icontains $prefix) {
            throw 'Uninstall left persistent PATH entry'
        }
    }
} finally {
    [Environment]::SetEnvironmentVariable('Path', $oldPath, 'User')
}
"#).unwrap();
    let result = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&script)
        .env("TEST_ROOT", root.path())
        .env("TEST_VERSION", env!("CARGO_PKG_VERSION"))
        .env("TEST_BINARY", env!("CARGO_BIN_EXE_lbc"))
        .env(
            "TEST_INSTALLER",
            std::env::current_dir().unwrap().join("install.ps1"),
        )
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
}
