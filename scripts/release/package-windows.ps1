# Package Windows x86_64: .zip (MSI is a documented follow-up, see docs/RELEASING.md).
#   pwsh scripts/release/package-windows.ps1 [-BinDir target\release] [-Dist dist]
param(
    [string]$BinDir = "target\release",
    [string]$Dist = "dist"
)
$ErrorActionPreference = "Stop"
$root = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$version = $env:FORGE_VERSION
if (-not $version) {
    $inSec = $false
    foreach ($line in Get-Content (Join-Path $root "Cargo.toml")) {
        if ($line -match '^\[workspace\.package\]') { $inSec = $true; continue }
        if ($line -match '^\[') { $inSec = $false }
        if ($inSec -and $line -match '^version\s*=\s*"([^"]+)"') { $version = $Matches[1]; break }
    }
}
if (-not $version) { throw "cannot determine workspace version" }

$stem = "forge3d-$version-windows-x86_64"
$bins = @("forge-studio", "forge", "forge-mcp", "forge-rpc-serve")
New-Item -ItemType Directory -Force -Path $Dist | Out-Null
$stage = Join-Path ([System.IO.Path]::GetTempPath()) ("forge3d-" + [guid]::NewGuid())
$pkg = Join-Path $stage $stem
New-Item -ItemType Directory -Force -Path (Join-Path $pkg "bin") | Out-Null

foreach ($b in $bins) {
    $exe = Join-Path $BinDir "$b.exe"
    if (-not (Test-Path $exe)) { throw "missing binary $exe" }
    # HOOK(177): sign each exe here; skipped when secrets are absent.
    & (Join-Path $PSScriptRoot "sign-hooks.ps1") -Path $exe
    Copy-Item $exe (Join-Path $pkg "bin")
}
Copy-Item (Join-Path $root "packaging\icons\forge3d.ico") $pkg
Get-ChildItem $root -Filter "LICENSE*" | Copy-Item -Destination $pkg
"Forge3D $version (Windows x86_64)`r`nRun bin\forge-studio.exe for the editor; bin\forge.exe is the command-line tool." |
    Set-Content (Join-Path $pkg "README.txt")

$zip = Join-Path $Dist "$stem.zip"
if (Test-Path $zip) { Remove-Item $zip }
Compress-Archive -Path $pkg -DestinationPath $zip
Remove-Item -Recurse -Force $stage
Write-Host "built $zip"
