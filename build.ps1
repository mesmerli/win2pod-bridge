# Build Win2Pod Bridge (release) and copy it to dist\win2pod.exe
# Requires: rustup GNU toolchain + MinGW-w64 (gcc, dlltool) on PATH.

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot
$mingw = "$env:USERPROFILE\mingw64\bin"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$mingw;$env:PATH"

# Close any running instances before building so files are not locked
Get-Process win2pod -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

Write-Host "Ensuring visual assets and icons are up to date..."
& (Join-Path $root "generate-assets.ps1")

Write-Host "Building win2pod (release)..."
Push-Location $root
try {
    cargo build -p homepod-cast-gui --bin win2pod --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$dest = Join-Path $dist "win2pod.exe"
Copy-Item (Join-Path $root "target\release\win2pod.exe") $dest -Force
Write-Host "Done -> $dest ($([math]::Round((Get-Item $dest).Length/1MB,2)) MB)"
