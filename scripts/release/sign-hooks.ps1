# Signing hook point for Windows (brief 177 fills this in).
# Skipped when the WINDOWS_CERT_PFX secret is absent, so unsigned builds work (DECISIONS D11).
param([Parameter(Mandatory = $true)][string]$Path)
if (-not $env:WINDOWS_CERT_PFX) {
    Write-Host "sign-hooks: Windows signing skipped (no secrets)"
    exit 0
}
# HOOK(177): decode the PFX, run signtool on $Path.
Write-Host "sign-hooks: Windows signing not implemented yet (brief 177)"
