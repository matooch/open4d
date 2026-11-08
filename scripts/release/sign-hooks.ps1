# Signing hook point for Windows (brief 177, docs/RELEASING.md).
# Skipped when the workflow did not prepare a certificate (WINDOWS_CERT_FILE), so unsigned
# builds work (DECISIONS D11). WINDOWS_CERT_PASSWORD and WINDOWS_TIMESTAMP_URL are optional.
param([Parameter(Mandatory = $true)][string]$Path)
if (-not $env:WINDOWS_CERT_FILE -or -not (Test-Path $env:WINDOWS_CERT_FILE)) {
    Write-Host "signing skipped: no Windows certificate"
    exit 0
}
$tsa = $env:WINDOWS_TIMESTAMP_URL
if (-not $tsa) { $tsa = "http://timestamp.digicert.com" }
$signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
    Sort-Object FullName | Select-Object -Last 1
if (-not $signtool) {
    Write-Host "signing skipped: signtool.exe not found"
    exit 0
}
$signArgs = @("sign", "/fd", "sha256", "/tr", $tsa, "/td", "sha256", "/f", $env:WINDOWS_CERT_FILE)
if ($env:WINDOWS_CERT_PASSWORD) { $signArgs += @("/p", $env:WINDOWS_CERT_PASSWORD) }
$signArgs += $Path
& $signtool.FullName @args
if ($LASTEXITCODE -ne 0) { throw "signtool failed for $Path" }
Write-Host "sign-hooks: signed $Path"
