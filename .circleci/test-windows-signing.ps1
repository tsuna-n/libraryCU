# Native API/certificate calls are mocked; ZIP/hash/ordering logic runs for real.
# These fixtures are NOT Authenticode, certificate trust, or production evidence.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
. "$PSScriptRoot/sign-windows-release.ps1"

$fixtureRoot = Join-Path ([IO.Path]::GetTempPath()) "LbcSigningTest-$([Guid]::NewGuid().ToString('N'))"
$script:scenario = "success"
$script:operations = New-Object 'System.Collections.Generic.List[string]'
$script:pfxCleaned = $false
$saved = @{}
foreach ($name in @("CIRCLE_TAG", "CIRCLE_SHA1", "CIRCLE_JOB", "CIRCLE_WORKFLOW_ID", "CIRCLE_PROJECT_USERNAME",
    "CIRCLE_PROJECT_REPONAME", "LBC_WINDOWS_CERT_THUMBPRINT", "LBC_WINDOWS_TIMESTAMP_URL", "LBC_WINDOWS_PFX_BASE64",
    "LBC_WINDOWS_PFX_PASSWORD", "LBC_WINDOWS_CERT_STORE", "LBC_TEST_BINARY", "LBC_CONFIG", "TMPDIR")) {
    $saved[$name] = [Environment]::GetEnvironmentVariable($name, "Process")
}

function Get-ReleaseVersion { return "0.5.0" }
function Assert-ReleaseIdentity { param($Version)
    if ($env:CIRCLE_TAG -ne "v$Version" -or $script:scenario -eq "wrong-source") { throw "Fixture source mismatch" }
}
function Resolve-SignTool { return "fixture-signtool" }
function Get-StoreCertificates { param($Store)
    $script:operations.Add("certificate")
    if ($script:scenario -eq "missing-certificate") { return @() }
    $certificate = [pscustomobject]@{
        Thumbprint = if ($script:scenario -eq "wrong-signer") { "B" * 40 } else { "A" * 40 }
        HasPrivateKey = ($script:scenario -ne "no-private-key")
        NotBefore = if ($script:scenario -eq "future-certificate") { [DateTime]::UtcNow.AddDays(1) } else { [DateTime]::UtcNow.AddDays(-1) }
        NotAfter = if ($script:scenario -eq "expired-certificate") { [DateTime]::UtcNow.AddDays(-1) } else { [DateTime]::UtcNow.AddDays(1) }
        EnhancedKeyUsageList = @([pscustomobject]@{ ObjectId = if ($script:scenario -eq "wrong-key-usage") { "1.2.3.4" } else { "1.3.6.1.5.5.7.3.3" } })
    }
    if ($script:scenario -eq "multiple-certificates") { return @($certificate, $certificate) }
    return $certificate
}
function Invoke-CheckedNative { param($Executable, $Arguments)
    $script:operations.Add($Arguments[0])
    if ($Arguments[0] -eq "sign") {
        if ($script:scenario -eq "sign-failure") { throw "Fixture signer failed" }
        if ($Arguments -cnotcontains "SHA256" -or $Arguments -cnotcontains "/sha1" -or
            $Arguments -cnotcontains "/tr" -or $Arguments -cnotcontains "/td") { throw "Unsafe signing options" }
        [IO.File]::AppendAllText($Arguments[-1], "AUTHENTICODE-FIXTURE")
    } elseif ($Arguments[0] -eq "verify") {
        if ($script:scenario -eq "verify-failure") { throw "Fixture verification failed" }
        foreach ($option in @("/pa", "/all", "/tw")) {
            if ($Arguments -cnotcontains $option) { throw "Missing verification option" }
        }
    }
}
function Get-AuthenticodeSignature { param($LiteralPath)
    $bytes = [IO.File]::ReadAllText($LiteralPath)
    if (-not $bytes.Contains("AUTHENTICODE-FIXTURE")) {
        return [pscustomobject]@{ Status = if ($script:scenario -eq "already-signed") { "Valid" } else { "NotSigned" } }
    }
    $script:operations.Add("embedded-verification")
    return [pscustomobject]@{
        Status = if ($script:scenario -eq "altered-signature") { "HashMismatch" } else { "Valid" }
        SignatureType = if ($script:scenario -eq "catalog-signature") { "Catalog" } else { "Authenticode" }
        SignerCertificate = [pscustomobject]@{ Thumbprint = if ($script:scenario -eq "embedded-wrong-signer") { "B" * 40 } else { "A" * 40 } }
        TimeStamperCertificate = if ($script:scenario -eq "missing-timestamp") { $null } else { [pscustomobject]@{ Thumbprint = "C" * 40 } }
    }
}
function Invoke-ReleaseCli { param($Binary, $Version)
    $script:operations.Add("signed-cli")
    if ($script:scenario -eq "cli-failure") { throw "Fixture CLI failed" }
    if (-not [IO.File]::ReadAllText($Binary).Contains("AUTHENTICODE-FIXTURE")) { throw "CLI ran unsigned bytes" }
}
function Import-ProvidedPfx { param($Store, $Pin)
    $script:operations.Add("pfx-import")
    if ($script:scenario -eq "malformed-pfx") { throw "Malformed fixture PFX" }
    return $null
}
function Remove-ProvidedPfx { param($Store)
    $script:pfxCleaned = $true
    $script:operations.Add("pfx-cleanup")
}

try {
    New-Item -ItemType Directory -Path $fixtureRoot | Out-Null
    $env:CIRCLE_TAG = "v0.5.0"; $env:CIRCLE_SHA1 = "1" * 40; $env:CIRCLE_JOB = "sign_windows_release"
    $env:CIRCLE_WORKFLOW_ID = "mock-native-workflow"; $env:CIRCLE_PROJECT_USERNAME = "tsuna-n"; $env:CIRCLE_PROJECT_REPONAME = "libraryCU"
    $env:LBC_WINDOWS_CERT_THUMBPRINT = "A" * 40; $env:LBC_WINDOWS_TIMESTAMP_URL = "https://timestamp.example.invalid"
    $env:LBC_WINDOWS_PFX_BASE64 = ""; $env:LBC_WINDOWS_PFX_PASSWORD = ""; $env:LBC_WINDOWS_CERT_STORE = ""
    $candidates = Join-Path $fixtureRoot "candidates"
    $package = "lbc-0.5.0-x86_64-pc-windows-msvc"
    $payload = Join-Path $fixtureRoot $package
    New-Item -ItemType Directory -Path $candidates, $payload | Out-Null
    foreach ($name in @("lbc.exe", "install.ps1", "README.md", "CHANGELOG.md", "LICENSE")) {
        [IO.File]::WriteAllText((Join-Path $payload $name), "unsigned-fixture")
    }
    $zip = Join-Path $candidates "$package.zip"
    Compress-Archive -LiteralPath $payload -DestinationPath $zip
    $utf8 = New-Object Text.UTF8Encoding($false)
    [IO.File]::WriteAllText("$zip.sha256", "$(Get-Sha256 $zip)  $package.zip`n", $utf8)
    $scenarios = @("success", "missing-certificate", "expired-certificate", "multiple-certificates", "no-private-key",
        "sign-failure", "verify-failure", "wrong-signer", "altered-signature", "missing-timestamp", "cli-failure",
        "already-signed", "wrong-source", "missing-pin", "short-pin", "insecure-timestamp", "checksum-mismatch",
        "malformed-pfx", "pfx-sign-failure", "pfx-success", "future-certificate", "wrong-key-usage", "catalog-signature", "embedded-wrong-signer")
    foreach ($case in $scenarios) {
        $script:scenario = $case; $script:operations.Clear(); $script:pfxCleaned = $false
        $env:LBC_WINDOWS_CERT_THUMBPRINT = "A" * 40; $env:LBC_WINDOWS_TIMESTAMP_URL = "https://timestamp.example.invalid"
        $env:LBC_WINDOWS_PFX_BASE64 = ""; $env:LBC_WINDOWS_PFX_PASSWORD = ""
        if ($case -eq "missing-pin") { $env:LBC_WINDOWS_CERT_THUMBPRINT = "" }
        if ($case -eq "short-pin") { $env:LBC_WINDOWS_CERT_THUMBPRINT = "A" * 16 }
        if ($case -eq "insecure-timestamp") { $env:LBC_WINDOWS_TIMESTAMP_URL = "http://timestamp.example.invalid" }
        if ($case -eq "checksum-mismatch") { [IO.File]::WriteAllText("$zip.sha256", "wrong", $utf8) }
        if ($case -in @("malformed-pfx", "pfx-sign-failure", "pfx-success")) {
            $env:LBC_WINDOWS_PFX_BASE64 = "MOCK PFX ONLY"; $env:LBC_WINDOWS_PFX_PASSWORD = "fixture-password"
        }
        if ($case -eq "pfx-sign-failure") { $script:scenario = "sign-failure" }
        $output = Join-Path $fixtureRoot "output-$case"
        $failed = $false
        $failureDetail = ""
        try { Invoke-WindowsRelease $candidates $output } catch { $failed = $true; $failureDetail = $_.Exception.Message }
        if ($case -in @("success", "pfx-success")) {
            if ($failed) { throw "Mocked signing success failed: ${case}: $failureDetail" }
            $receipt = Get-Content -Raw -LiteralPath (Join-Path $output "lbc-0.5.0.windows-signing.json") | ConvertFrom-Json
            if ($receipt.mode -ne "production" -or -not $receipt.verification.packagedBinary -or
                $receipt.archives[0].sha256 -cne (Get-Sha256 (Join-Path $output "$package.zip"))) { throw "Invalid mock receipt" }
            if ($script:operations.IndexOf("sign") -ge $script:operations.IndexOf("signed-cli") -or
                $script:operations.IndexOf("verify") -ge $script:operations.IndexOf("signed-cli")) { throw "Signing/verification order broken" }
            if ($case -eq "pfx-success" -and -not $script:pfxCleaned) { throw "Temporary PFX store not cleaned" }
        } else {
            if (-not $failed) { throw "Unsafe mocked signing case succeeded: $case" }
            if (Test-Path -LiteralPath (Join-Path $output "lbc-0.5.0.windows-signing.json")) { throw "Failed signing emitted success record" }
            if ($case -in @("malformed-pfx", "pfx-sign-failure") -and -not $script:pfxCleaned) { throw "Failed PFX flow not cleaned" }
        }
        [IO.File]::WriteAllText("$zip.sha256", "$(Get-Sha256 $zip)  $package.zip`n", $utf8)
    }
    # Exercise the real archive policy independently of mocked signing APIs.
    $badZip = Join-Path $fixtureRoot "bad.zip"
    $archive = [IO.Compression.ZipFile]::Open($badZip, [IO.Compression.ZipArchiveMode]::Create)
    try {
        $entry = $archive.CreateEntry("../escape.exe")
        $writer = New-Object IO.StreamWriter($entry.Open()); $writer.Write("unsafe"); $writer.Dispose()
    } finally { $archive.Dispose() }
    $rejected = $false
    try { Assert-ZipLayout $badZip $package } catch { $rejected = $true }
    if (-not $rejected) { throw "Traversal ZIP accepted" }
    $duplicateZip = Join-Path $fixtureRoot "duplicate-normalized.zip"
    Copy-Item -LiteralPath $zip -Destination $duplicateZip
    $archive = [IO.Compression.ZipFile]::Open($duplicateZip, [IO.Compression.ZipArchiveMode]::Update)
    try {
        # In PS5 the existing spelling has a backslash; in PS7 it has a slash.
        $existing = @($archive.Entries | Where-Object { $_.FullName.Replace('\', '/') -eq "$package/lbc.exe" })[0]
        $alternate = if ($existing.FullName.Contains('\')) { "$package/lbc.exe" } else { "$package\lbc.exe" }
        $entry = $archive.CreateEntry($alternate)
        $writer = New-Object IO.StreamWriter($entry.Open()); $writer.Write("duplicate"); $writer.Dispose()
    } finally { $archive.Dispose() }
    $rejected = $false
    try { Assert-ZipLayout $duplicateZip $package } catch { $rejected = $true }
    if (-not $rejected) { throw "Duplicate normalized ZIP member accepted" }
    Write-Host "Windows signing fixtures: 26 passed (MOCK native/certificate calls; no production signature evidence)"
} finally {
    foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name], "Process") }
    if (Test-Path -LiteralPath $fixtureRoot) { Remove-Item -LiteralPath $fixtureRoot -Recurse -Force }
}
