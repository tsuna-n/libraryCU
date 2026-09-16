# Real disposable certificate/PFX lifecycle on the ephemeral Windows CI VM.
# It is self-signed and never installed as a trusted root or used for production.
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
if ($env:OS -ne "Windows_NT" -or $env:CIRCLECI -ne "true") { throw "Disposable PFX test requires the Windows CI VM" }
. "$PSScriptRoot/sign-windows-release.ps1"
$certificate = $null; $collection = $null
$store = "LbcRelease-$([Guid]::NewGuid().ToString('N'))"
$savedPfx = $env:LBC_WINDOWS_PFX_BASE64; $savedPassword = $env:LBC_WINDOWS_PFX_PASSWORD
try {
    Resolve-SignTool | Out-Null
    $certificate = New-SelfSignedCertificate -Type CodeSigningCert -KeyExportPolicy Exportable `
        -Subject "CN=libraryCube disposable CI fixture-$([Guid]::NewGuid())" `
        -CertStoreLocation "Cert:\CurrentUser\My" -NotAfter ([DateTime]::Now.AddDays(1))
    $bytes = $certificate.Export([Security.Cryptography.X509Certificates.X509ContentType]::Pfx, "fixture-password")
    $env:LBC_WINDOWS_PFX_BASE64 = [Convert]::ToBase64String($bytes)
    [Array]::Clear($bytes, 0, $bytes.Length)
    $env:LBC_WINDOWS_PFX_PASSWORD = "fixture-password"
    $collection = Import-ProvidedPfx $store $certificate.Thumbprint
    $imported = Get-ReleaseCertificate $store $certificate.Thumbprint
    if (-not $imported.HasPrivateKey) { throw "Transient PFX lost its signing key before signing" }
    Remove-ProvidedPfx $store
    foreach ($item in $collection) { $item.Dispose() }; $collection = $null
    if (Test-Path -LiteralPath "Cert:\CurrentUser\$store") { throw "Temporary PFX store retained" }
    $failed = $false
    try { Import-ProvidedPfx $store ("0" * 40) | Out-Null } catch { $failed = $true }
    if (-not $failed -or (Test-Path -LiteralPath "Cert:\CurrentUser\$store")) { throw "Mismatched PFX imported or retained" }
    $env:LBC_WINDOWS_PFX_BASE64 = "not-base64"
    $failed = $false
    try { Import-ProvidedPfx $store $certificate.Thumbprint | Out-Null } catch { $failed = $true }
    if (-not $failed -or (Test-Path -LiteralPath "Cert:\CurrentUser\$store")) { throw "Malformed PFX imported or retained" }
    Write-Host "Windows disposable PFX lifecycle: 3 passed (self-signed TEST certificate; no production trust evidence)"
} finally {
    try { Remove-ProvidedPfx $store }
    finally {
        if ($null -ne $collection) { foreach ($item in $collection) { $item.Dispose() } }
        if ($null -ne $certificate) {
            Remove-Item -LiteralPath "Cert:\CurrentUser\My\$($certificate.Thumbprint)" -DeleteKey -Force
            $certificate.Dispose()
        }
        $env:LBC_WINDOWS_PFX_BASE64 = $savedPfx; $env:LBC_WINDOWS_PFX_PASSWORD = $savedPassword
    }
}
