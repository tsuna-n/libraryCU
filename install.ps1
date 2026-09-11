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
    [switch]$Uninstall
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

$bundledBinary = Join-Path $scriptRoot "lbc.exe"
if ($sourceTree -and -not $NoBuild) {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust was not found. Install it from https://rustup.rs/ or use an official release archive."
    }
    Write-Host "Building libraryCube in release mode..."
    & cargo build --locked --release --manifest-path $manifestPath
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE."
    }
}

$sourceBinary = if ($sourceTree) {
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
    if (($env:Path -split ";") -inotcontains $installDir) {
        $env:Path = "$installDir;$env:Path"
    }
}

Write-Host "Installed $version to $targetBinary"
if ($NoPathUpdate) {
    Write-Host "PATH was not changed because -NoPathUpdate was supplied."
} else {
    Write-Host "Added $installDir to the $pathScope PATH. Open a new terminal, then run: lbc --help"
}
