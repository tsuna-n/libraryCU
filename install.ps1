<#
.SYNOPSIS
Installs the libraryCube (lbc) command on Windows.

.DESCRIPTION
An official release archive contains install.ps1 and a prebuilt lbc.exe, so
Rust is not required. In a source checkout the script builds the release binary
unless -NoBuild is supplied.
#>

[CmdletBinding()]
param(
    [string]$Prefix,
    [switch]$System,
    [switch]$NoBuild,
    [switch]$NoPathUpdate,
    [switch]$Uninstall,
    [string]$Archive,
    [string]$Sha256
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-NormalizedPath([string]$Value) {
    if ([string]::IsNullOrWhiteSpace($Value)) {
        return ""
    }
    $expanded = [Environment]::ExpandEnvironmentVariables($Value.Trim())
    try {
        return [IO.Path]::GetFullPath($expanded).TrimEnd([char[]]"\/")
    } catch {
        return $expanded.TrimEnd([char[]]"\/")
    }
}

function Update-Path([string]$Directory, [string]$Scope, [bool]$Remove) {
    $current = [Environment]::GetEnvironmentVariable("Path", $Scope)
    $entries = @($current -split ";" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $normalizedDirectory = Get-NormalizedPath $Directory
    $kept = @($entries | Where-Object {
        (Get-NormalizedPath $_) -ine $normalizedDirectory
    })

    if (-not $Remove) {
        $kept += $Directory
    }
    [Environment]::SetEnvironmentVariable("Path", ($kept -join ";"), $Scope)
}

function Test-PathEntry([string]$Value, [string]$Directory) {
    $normalized = Get-NormalizedPath $Directory
    return @($Value -split ";" | Where-Object {
        (Get-NormalizedPath $_) -ieq $normalized
    }).Count -gt 0
}

function Read-VerifiedArchive([string]$Path, [string]$Digest) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf) -or
        ((Get-Item -LiteralPath $Path).Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Archive must be a regular file: $Path"
    }
    $temporary = Join-Path ([IO.Path]::GetTempPath()) ("lbc-install-" + [Guid]::NewGuid().ToString("N"))
    New-Item -ItemType Directory -Path $temporary | Out-Null
    try {
        # Hash the private copy we will read, so a changed download cannot race verification.
        $copy = Join-Path $temporary "release.zip"
        Copy-Item -LiteralPath $Path -Destination $copy
        if ((Get-FileHash -Algorithm SHA256 -LiteralPath $copy).Hash -ine $Digest) {
            throw "SHA-256 mismatch. Installation was not changed."
        }
        $architecture = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
        if ($env:OS -ne "Windows_NT" -or $architecture -ine "AMD64") {
            throw "No supported release archive for this platform/architecture. Expected Windows x86_64."
        }
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [IO.Compression.ZipFile]::OpenRead($copy)
        try {
            if ($zip.Entries.Count -eq 0) { throw "Empty release archive." }
            $first = $zip.Entries[0].FullName.Replace('\', '/')
            $package = ($first -split '/')[0]
            if ($package -cnotmatch '^lbc-([0-9]+\.[0-9]+\.[0-9]+)-x86_64-pc-windows-msvc$') {
                throw "Archive package does not match this platform (Windows x86_64)."
            }
            $packageVersion = $Matches[1]
            $expected = @("lbc.exe", "install.ps1", "README.md", "CHANGELOG.md", "LICENSE") |
                ForEach-Object { "$package/$_" }
            $seen = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
            foreach ($entry in $zip.Entries) {
                # Match the release policy, including Windows PowerShell 5 ZIP separators.
                $name = $entry.FullName.Replace('\', '/')
                $identity = $name.TrimEnd('/')
                if (-not $seen.Add($identity)) { throw "Duplicate ZIP member: $name" }
                $unixType = (($entry.ExternalAttributes -shr 16) -band 0xf000)
                $dosAttributes = $entry.ExternalAttributes -band 0xffff
                if (($dosAttributes -band 0x408) -ne 0) { throw "Unsafe/unexpected ZIP member: $name" }
                if ($name -ceq "$package/") {
                    if ($entry.Length -ne 0 -or $unixType -notin @(0, 0x4000)) { throw "Unsafe ZIP directory." }
                    continue
                }
                if ($expected -cnotcontains $name -or $unixType -notin @(0, 0x8000) -or
                    ($dosAttributes -band 0x10) -ne 0 -or $entry.Length -le 0 -or $entry.Length -gt 128MB) {
                    throw "Unsafe, unexpected, or oversized ZIP member: $name"
                }
            }
            foreach ($name in $expected) {
                if (-not $seen.Contains($name)) { throw "Incomplete Windows package: missing $name" }
            }
            $binary = Join-Path $temporary "lbc.exe"
            foreach ($entry in $zip.Entries) {
                if ($entry.FullName.Replace('\', '/') -ceq "$package/") { continue }
                $inputStream = $entry.Open()
                try {
                    # Read every member, but only materialize the binary; never run install.ps1 from the ZIP.
                    if ($entry.FullName.Replace('\', '/') -ceq "$package/lbc.exe") {
                        $outputStream = [IO.File]::Create($binary)
                        try { $inputStream.CopyTo($outputStream) } finally { $outputStream.Dispose() }
                    } else {
                        $inputStream.CopyTo([IO.Stream]::Null)
                    }
                } finally { $inputStream.Dispose() }
            }
        } finally { $zip.Dispose() }
        $stream = [IO.File]::OpenRead($binary)
        try {
            if ($stream.ReadByte() -ne 0x4d -or $stream.ReadByte() -ne 0x5a) {
                throw "Archive does not contain a native Windows executable."
            }
        } finally { $stream.Dispose() }
        $version = (& $binary --version | Out-String).Trim()
        if ($LASTEXITCODE -ne 0 -or $version -cne "lbc $packageVersion") {
            throw "Archive binary version verification failed. Installation was not changed."
        }
        return $temporary
    } catch {
        Remove-Item -LiteralPath $temporary -Recurse -Force
        throw
    }
}

if ($Archive -or $Sha256) {
    if (-not $Archive -or $Sha256 -notmatch '^[0-9a-fA-F]{64}$' -or $Uninstall) {
        throw "-Archive and -Sha256 require a file and an independently trusted 64-character hex digest; do not combine them with -Uninstall."
    }
}

if ($System -and -not [string]::IsNullOrWhiteSpace($Prefix)) {
    throw "Use either -System or -Prefix, not both."
}

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$manifestPath = Join-Path $scriptRoot "Cargo.toml"
$sourceTree = Test-Path -LiteralPath $manifestPath -PathType Leaf

if ($System) {
    if (-not (Test-Administrator)) {
        throw "-System requires an elevated PowerShell session."
    }
    $installDir = Join-Path $env:ProgramFiles "libraryCube\bin"
    $pathScope = "Machine"
} elseif (-not [string]::IsNullOrWhiteSpace($Prefix)) {
    $installDir = [IO.Path]::GetFullPath(
        [Environment]::ExpandEnvironmentVariables($Prefix)
    )
    $pathScope = "User"
} else {
    $installDir = Join-Path $env:LOCALAPPDATA "Programs\libraryCube\bin"
    $pathScope = "User"
}

$targetBinary = Join-Path $installDir "lbc.exe"

if ($Uninstall) {
    if (Test-Path -LiteralPath $targetBinary -PathType Leaf) {
        Remove-Item -LiteralPath $targetBinary -Force
        Write-Host "Removed $targetBinary"
    } else {
        Write-Warning "Binary not found at $targetBinary. Nothing to uninstall."
    }
    if (-not $NoPathUpdate) {
        Update-Path -Directory $installDir -Scope $pathScope -Remove $true
        Write-Host "Removed $installDir from the $pathScope PATH."
    }
    exit 0
}

$archiveTemporary = $null
try {
if ($Archive) {
    $archiveTemporary = Read-VerifiedArchive -Path $Archive -Digest $Sha256
}
$bundledBinary = Join-Path $scriptRoot "lbc.exe"
if ($sourceTree -and -not $NoBuild -and -not $Archive) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust was not found. Install it from https://rustup.rs/ or use an official release archive."
    }
    Write-Host "Building libraryCube in release mode..."
    & cargo build --locked --release --manifest-path $manifestPath
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE."
    }
}

$sourceBinary = if ($Archive) {
    Join-Path $archiveTemporary "lbc.exe"
} elseif ($sourceTree) {
    Join-Path $scriptRoot "target\release\lbc.exe"
} else {
    $bundledBinary
}
if (-not (Test-Path -LiteralPath $sourceBinary -PathType Leaf) -and
    (Test-Path -LiteralPath $bundledBinary -PathType Leaf)) {
    $sourceBinary = $bundledBinary
}
if (-not (Test-Path -LiteralPath $sourceBinary -PathType Leaf)) {
    throw "No lbc.exe was found. Build the source or use an official release archive."
}

New-Item -ItemType Directory -Path $installDir -Force | Out-Null
if ((Get-NormalizedPath $sourceBinary) -ine (Get-NormalizedPath $targetBinary)) {
    Copy-Item -LiteralPath $sourceBinary -Destination $targetBinary -Force
}

$version = (& $targetBinary --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or -not $version.StartsWith("lbc ")) {
    throw "Installation verification failed for $targetBinary."
}

if (-not $NoPathUpdate) {
    Update-Path -Directory $installDir -Scope $pathScope -Remove $false
    if (-not (Test-PathEntry ([Environment]::GetEnvironmentVariable("Path", $pathScope)) $installDir)) {
        throw "Could not verify $installDir in the persistent $pathScope PATH."
    }
    if (-not (Test-PathEntry $env:Path $installDir)) {
        $env:Path = "$installDir;$env:Path"
    }
}

Write-Host "Installed $version to $targetBinary"
if ($NoPathUpdate) {
    Write-Host "PATH was not changed because -NoPathUpdate was supplied."
    if (-not (Test-PathEntry $env:Path $installDir)) {
        $quotedDirectory = $installDir.Replace("'", "''")
        Write-Host "For this PowerShell session, run:"
        Write-Host ('  $env:Path = ''' + $quotedDirectory + ';'' + $env:Path')
    }
} else {
    Write-Host "Verified $installDir in the persistent $pathScope PATH."
    Write-Host "Close and reopen your terminal application (including Windows Terminal or your editor), then run the commands below. A new tab in an existing application may keep its old PATH."
}
Write-Host "Quick start:"
Write-Host "  lbc setup"
Write-Host "  lbc learn"
Write-Host '  lbc search "your problem"'
Write-Host '  lbc explain build.log'
} finally {
    if ($archiveTemporary) { Remove-Item -LiteralPath $archiveTemporary -Recurse -Force }
}
