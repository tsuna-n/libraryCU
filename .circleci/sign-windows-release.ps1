[CmdletBinding()]
param([string]$CandidateDir = "dist", [string]$OutputDir = "production-dist")
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Invoke-CheckedNative {
    param([string]$Executable, [string[]]$Arguments)
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) { throw "Native command failed or warned: $Executable ($LASTEXITCODE)" }
}

function Get-ReleaseVersion {
    $metadata = (& cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json)
    if ($LASTEXITCODE -ne 0) { throw "Cannot read locked Cargo metadata" }
    $version = @($metadata.packages | Where-Object name -eq "librarycube").version
    if ([string]::IsNullOrWhiteSpace($version)) { throw "Missing package version" }
    return $version
}

function Assert-ReleaseIdentity {
    param([string]$Version)
    if ($env:CIRCLE_TAG -ne "v$Version" -or $env:CIRCLE_JOB -ne "sign_windows_release" -or
        [string]::IsNullOrWhiteSpace($env:CIRCLE_WORKFLOW_ID) -or
        $env:CIRCLE_SHA1 -cnotmatch '^([0-9a-f]{40}|[0-9a-f]{64})$' -or
        $env:CIRCLE_PROJECT_USERNAME -ne "tsuna-n" -or $env:CIRCLE_PROJECT_REPONAME -ne "libraryCU") {
        throw "Production native signing requires the exact gated repository/version/tag/workflow"
    }
    $headSha = (& git rev-parse HEAD | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $headSha -cne $env:CIRCLE_SHA1) { throw "Native source SHA mismatch" }
}

function Resolve-SignTool {
    if ($env:LBC_SIGNTOOL_PATH) {
        $path = $env:LBC_SIGNTOOL_PATH
    } else {
        $sdkRoot = Join-Path ${env:ProgramFiles(x86)} "Windows Kits\10\bin"
        $tools = @(Get-ChildItem -LiteralPath $sdkRoot -Directory |
            Where-Object Name -match '^10\.[0-9]+\.[0-9]+\.[0-9]+$' |
            Sort-Object { [version]$_.Name } -Descending |
            ForEach-Object { Join-Path $_.FullName "x64\signtool.exe" } |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf })
        if ($tools.Count -eq 0) { throw "Windows SDK signtool.exe is required" }
        $path = $tools[0]
    }
    Assert-RegularFile $path
    return $path
}

function Assert-RegularFile {
    param([string]$Path)
    $item = Get-Item -LiteralPath $Path -Force
    if ($item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or $item.Length -le 0) {
        throw "Unsafe or empty release file: $Path"
    }
}

function Get-Sha256 {
    param([string]$Path)
    Assert-RegularFile $Path
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Assert-ZipLayout {
    param([string]$Path, [string]$Package)
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        $expected = @("lbc.exe", "install.ps1", "README.md", "CHANGELOG.md", "LICENSE") |
            ForEach-Object { "$Package/$_" }
        $seen = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
        foreach ($entry in $zip.Entries) {
            # Windows PowerShell 5 Compress-Archive uses backslash members.
            # Canonicalize BEFORE exact-name and duplicate checks; no traversal
            # or second spelling of the same member may pass.
            $name = $entry.FullName.Replace('\', '/')
            if (-not $seen.Add($name)) { throw "Duplicate ZIP member" }
            $unixType = (($entry.ExternalAttributes -shr 16) -band 0xf000)
            $dosAttributes = $entry.ExternalAttributes -band 0xffff
            if (($dosAttributes -band 0x408) -ne 0) { throw "Unsafe/unexpected ZIP member: $name" }
            if ($name -eq "$Package/") {
                if ($entry.Length -ne 0 -or $unixType -notin @(0, 0x4000)) { throw "Unsafe ZIP directory" }
                continue
            }
            if ($expected -cnotcontains $name -or $unixType -notin @(0, 0x8000) -or ($dosAttributes -band 0x10) -ne 0 -or
                $entry.Length -le 0 -or $entry.Length -gt 128MB) { throw "Unsafe/unexpected ZIP member: $name" }
        }
        foreach ($name in $expected) { if (-not $seen.Contains($name)) { throw "Incomplete Windows package" } }
    } finally { $zip.Dispose() }
}

function Get-StoreCertificates {
    param([string]$Store)
    Get-ChildItem -LiteralPath "Cert:\CurrentUser\$Store"
}

function Get-ReleaseCertificate {
    param([string]$Store, [string]$Pin)
    $certificates = @(Get-StoreCertificates $Store |
        Where-Object { $_.Thumbprint -ceq $Pin })
    if ($certificates.Count -ne 1) { throw "Expected exactly one pinned code-signing certificate" }
    $certificate = $certificates[0]
    if (-not $certificate.HasPrivateKey -or $certificate.NotBefore.ToUniversalTime() -gt [DateTime]::UtcNow -or
        $certificate.NotAfter.ToUniversalTime() -le [DateTime]::UtcNow -or
        @($certificate.EnhancedKeyUsageList | Where-Object ObjectId -eq "1.3.6.1.5.5.7.3.3").Count -eq 0) {
        throw "Code-signing certificate is expired, unavailable, or has wrong key usage"
    }
    return $certificate
}

function Assert-Authenticode {
    param([string]$Tool, [string]$Binary, [string]$Pin)
    Invoke-CheckedNative $Tool @("verify", "/pa", "/all", "/tw", "/v", $Binary)
    $signature = Get-AuthenticodeSignature -LiteralPath $Binary
    if ($signature.Status -ne "Valid" -or $null -eq $signature.SignerCertificate -or
        $signature.SignerCertificate.Thumbprint -cne $Pin -or $null -eq $signature.TimeStamperCertificate -or
        $signature.SignatureType -ne "Authenticode") {
        throw "Authenticode signer, embedded signature, trust, or timestamp mismatch"
    }
}

function Invoke-ReleaseCli {
    param([string]$Binary, [string]$Version)
    $reported = (& $Binary --version | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $reported -ne "lbc $Version") { throw "Signed binary version mismatch" }
    $env:LBC_TEST_BINARY = $Binary
    Invoke-CheckedNative "cargo" @("test", "--locked", "--test", "cli")
}

function Import-ProvidedPfx {
    param([string]$Store, [string]$Pin)
    $collection = New-Object Security.Cryptography.X509Certificates.X509Certificate2Collection
    $bytes = $null
    try {
        $bytes = [Convert]::FromBase64String($env:LBC_WINDOWS_PFX_BASE64)
        # Do not use PersistKeySet. Keep this collection alive until signing is
        # finished; disposal also removes transient keys on failed imports.
        $flags = [Security.Cryptography.X509Certificates.X509KeyStorageFlags]::UserKeySet
        $collection.Import($bytes, $env:LBC_WINDOWS_PFX_PASSWORD, $flags)
        $signers = @($collection | Where-Object HasPrivateKey)
        if ($signers.Count -ne 1 -or $signers[0].Thumbprint -cne $Pin) {
            throw "Provided PFX must contain exactly one matching private signing identity"
        }
        $storeObject = New-Object Security.Cryptography.X509Certificates.X509Store($Store, "CurrentUser")
        try {
            $storeObject.Open([Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
            $storeObject.Add($signers[0])
        } finally { $storeObject.Close() }
        return ,$collection
    } catch {
        foreach ($certificate in $collection) { $certificate.Dispose() }
        throw
    } finally {
        if ($null -ne $bytes) { [Array]::Clear($bytes, 0, $bytes.Length) }
    }
}

function Remove-ProvidedPfx {
    param([string]$Store)
    if ($Store -cnotmatch '^LbcRelease-[0-9a-f]{32}$') { throw "Invalid temporary certificate store" }
    if (Test-Path -LiteralPath "Cert:\CurrentUser\$Store") {
        Get-ChildItem -LiteralPath "Cert:\CurrentUser\$Store" | ForEach-Object {
            Remove-Item -LiteralPath $_.PSPath -DeleteKey -Force
        }
        Remove-Item -LiteralPath "HKCU:\Software\Microsoft\SystemCertificates\$Store" -Recurse -Force
    }
}

function Invoke-WindowsRelease {
    param([string]$Candidates, [string]$Output)
    $version = Get-ReleaseVersion
    Assert-ReleaseIdentity $version
    $pin = $env:LBC_WINDOWS_CERT_THUMBPRINT
    if ($pin -cnotmatch '^[0-9A-F]{40}$') { throw "EXTERNAL CREDENTIAL REQUIRED: full Windows certificate thumbprint" }
    $timestamp = $null
    if (-not [Uri]::TryCreate($env:LBC_WINDOWS_TIMESTAMP_URL, [UriKind]::Absolute, [ref]$timestamp) -or
        $timestamp.Scheme -ne "https" -or $timestamp.UserInfo -or $timestamp.Fragment) {
        throw "EXTERNAL CREDENTIAL REQUIRED: approved HTTPS RFC 3161 timestamp URL"
    }
    $tool = Resolve-SignTool
    $package = "lbc-$version-x86_64-pc-windows-msvc"
    $archive = "$package.zip"
    $candidateZip = Join-Path $Candidates $archive
    $candidateChecksum = "$candidateZip.sha256"
    Assert-RegularFile $candidateChecksum
    if ([IO.File]::ReadAllText((Resolve-Path $candidateChecksum)).Replace("`r", "").TrimEnd("`n") -cne
        "$(Get-Sha256 $candidateZip)  $archive") { throw "Candidate Windows checksum mismatch" }
    Assert-ZipLayout $candidateZip $package
    New-Item -ItemType Directory -Force -Path $Output | Out-Null
    if ((Get-Item -LiteralPath $Output).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "Unsafe native output directory"
    }
    foreach ($name in @($archive, "$archive.sha256", "lbc-$version.windows-signing.json")) {
        if (Test-Path -LiteralPath (Join-Path $Output $name)) { throw "Refusing to replace existing native output" }
    }
    $nativeRoot = Join-Path ([IO.Path]::GetTempPath()) "LbcNative-$([Guid]::NewGuid().ToString('N'))"
    $temporaryStore = ""
    $providedCollection = $null
    $savedEnvironment = @{}
    foreach ($name in @("LBC_TEST_BINARY", "LBC_CONFIG", "XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_CACHE_HOME", "TMPDIR")) {
        $savedEnvironment[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
    }
    try {
        New-Item -ItemType Directory -Path $nativeRoot | Out-Null
        $packageArea = Join-Path $nativeRoot "package"
        Expand-Archive -LiteralPath $candidateZip -DestinationPath $packageArea
        $packageRoot = Join-Path $packageArea $package
        $binary = Join-Path $packageRoot "lbc.exe"
        Assert-RegularFile $binary
        if ((Get-AuthenticodeSignature -LiteralPath $binary).Status -ne "NotSigned") {
            throw "Expected an unsigned candidate, not a replacement for an existing native signature"
        }
        $store = if ($env:LBC_WINDOWS_CERT_STORE) { $env:LBC_WINDOWS_CERT_STORE } else { "My" }
        if ($store -cnotmatch '^[A-Za-z0-9-]+$') { throw "Invalid signing certificate store" }
        if ($env:LBC_WINDOWS_PFX_BASE64) {
            if ([string]::IsNullOrWhiteSpace($env:LBC_WINDOWS_PFX_PASSWORD)) { throw "Missing masked PFX password" }
            $temporaryStore = "LbcRelease-$([Guid]::NewGuid().ToString('N'))"
            $store = $temporaryStore
            $providedCollection = Import-ProvidedPfx $store $pin
        }
        $certificate = Get-ReleaseCertificate $store $pin
        Invoke-CheckedNative $tool @("sign", "/s", $store, "/sha1", $pin, "/fd", "SHA256",
            "/tr", $timestamp.AbsoluteUri, "/td", "SHA256", $binary)
        Assert-Authenticode $tool $binary $pin
        $fixtures = Join-Path $nativeRoot "fixtures"
        New-Item -ItemType Directory -Force -Path "$fixtures\config", "$fixtures\data", "$fixtures\cache", "$fixtures\tmp" | Out-Null
        $env:LBC_CONFIG = "$fixtures\config.toml"
        $env:XDG_CONFIG_HOME = "$fixtures\config"
        $env:XDG_DATA_HOME = "$fixtures\data"
        $env:XDG_CACHE_HOME = "$fixtures\cache"
        $env:TMPDIR = "$fixtures\tmp"
        Invoke-ReleaseCli $binary $version
        Assert-Authenticode $tool $binary $pin
        $binaryHash = Get-Sha256 $binary
        $finalZip = Join-Path $nativeRoot $archive
        Compress-Archive -LiteralPath $packageRoot -DestinationPath $finalZip
        Assert-ZipLayout $finalZip $package
        $verificationRoot = Join-Path $nativeRoot "verify"
        Expand-Archive -LiteralPath $finalZip -DestinationPath $verificationRoot
        $packagedBinary = Join-Path $verificationRoot "$package\lbc.exe"
        Assert-Authenticode $tool $packagedBinary $pin
        if ((Get-Sha256 $packagedBinary) -cne $binaryHash) { throw "Packaged signed binary changed" }
        $hash = Get-Sha256 $finalZip
        $receipt = [ordered]@{
            schemaVersion = 1; kind = "librarycube/native-verification/v1"; platform = "windows"; mode = "production"
            repository = "tsuna-n/libraryCU"; sourceSha = $env:CIRCLE_SHA1; version = $version; tag = "v$version"
            workflowId = $env:CIRCLE_WORKFLOW_ID; jobName = "sign_windows_release"
            signer = @{ certificateSha1 = $certificate.Thumbprint }
            binary = @{ path = "$package/lbc.exe"; sha256 = $binaryHash }
            archives = @(@{ name = $archive; sha256 = $hash })
            verification = @{ signature = $true; timestamp = $true; releaseCli = $true; packagedBinary = $true }
        }
        $utf8 = New-Object Text.UTF8Encoding($false)
        [IO.File]::WriteAllText((Join-Path $nativeRoot "receipt.json"), ($receipt | ConvertTo-Json -Depth 8), $utf8)
        Move-Item -LiteralPath $finalZip -Destination (Join-Path $Output $archive)
        [IO.File]::WriteAllText((Join-Path $Output "$archive.sha256"), "$hash  $archive`n", $utf8)
        Move-Item -LiteralPath (Join-Path $nativeRoot "receipt.json") -Destination (Join-Path $Output "lbc-$version.windows-signing.json")
        Write-Host "Pinned Authenticode, timestamp, signed CLI, and final ZIP verified for $env:CIRCLE_SHA1"
    } finally {
        foreach ($name in $savedEnvironment.Keys) { [Environment]::SetEnvironmentVariable($name, $savedEnvironment[$name], "Process") }
        try { if ($temporaryStore) { Remove-ProvidedPfx $temporaryStore } }
        finally {
            try { if ($null -ne $providedCollection) { foreach ($certificate in $providedCollection) { $certificate.Dispose() } } }
            finally { if (Test-Path -LiteralPath $nativeRoot) { Remove-Item -LiteralPath $nativeRoot -Recurse -Force } }
        }
    }
}

if ($MyInvocation.InvocationName -ne '.') { Invoke-WindowsRelease $CandidateDir $OutputDir }
