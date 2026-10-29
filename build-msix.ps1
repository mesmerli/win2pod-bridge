# ==============================================================================
# Build MSIX Package for Win2Pod Bridge (Microsoft Store & Sideloading)
# ==============================================================================
# Package Name:    mesmerli.Win2PodBridge
# Publisher:       CN=B50F36C9-7E5E-4652-A715-0185D35ADE64
# PublisherName:   mesmerli
# PFN:             mesmerli.Win2PodBridge_6t1y50ntn36fm
# Store ID:        9N9ZC9MB9SB3
# Version:         0.1.0.0
# ==============================================================================

$ErrorActionPreference = "Stop"
$root = $PSScriptRoot

# Close any running instances before building so files are not locked
Get-Process win2pod -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

Write-Host "=== 1. Generating Store Visual Assets & App Icons ===" -ForegroundColor Cyan
& (Join-Path $root "generate-assets.ps1")

Write-Host "=== 2. Building Release Executable ===" -ForegroundColor Cyan
Push-Location $root
try {
    $mingw = "$env:USERPROFILE\mingw64\bin"
    $env:PATH = "$env:USERPROFILE\.cargo\bin;$mingw;$env:PATH"
    cargo build -p homepod-cast-gui --bin win2pod --release
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally {
    Pop-Location
}

$dist = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$targetExe = Join-Path $root "target\release\win2pod.exe"
$distExe = Join-Path $dist "win2pod.exe"
Copy-Item $targetExe $distExe -Force

Write-Host "=== 3. Staging Packaging Files ===" -ForegroundColor Cyan
$staging = Join-Path $dist "msix_staging"
Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $staging | Out-Null

Copy-Item $distExe (Join-Path $staging "win2pod.exe") -Force
Copy-Item (Join-Path $root "package\msix\AppxManifest.xml") (Join-Path $staging "AppxManifest.xml") -Force
Copy-Item (Join-Path $root "package\msix\Assets") (Join-Path $staging "Assets") -Recurse -Force

Write-Host "=== 4. Packing MSIX with MakeAppx ===" -ForegroundColor Cyan
$makeappx = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\makeappx.exe"
if (-not (Test-Path $makeappx)) {
    throw "makeappx.exe not found at $makeappx"
}
$outMsix = Join-Path $dist "mesmerli.Win2PodBridge_0.1.0.0_x64.msix"
& $makeappx pack /v /o /d $staging /p $outMsix

Write-Host "=== 5. Signing MSIX with Developer Certificate ===" -ForegroundColor Cyan
$certSubject = "CN=B50F36C9-7E5E-4652-A715-0185D35ADE64"
$existingCert = Get-ChildItem -Path Cert:\CurrentUser\My | Where-Object { $_.Subject -eq $certSubject }
if (-not $existingCert) {
    $devCert = New-SelfSignedCertificate -Type Custom -Subject $certSubject `
        -KeyUsage DigitalSignature `
        -FriendlyName "Win2Pod Bridge Dev Cert" `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3")
} else {
    $devCert = $existingCert[0]
}

# Export certificate to dist for local sideloading installation
$certPath = Join-Path $dist "Win2PodBridge_DevCert.cer"
Export-Certificate -Cert $devCert -FilePath $certPath -Force | Out-Null

$signtool = "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\signtool.exe"
if (Test-Path $signtool) {
    & $signtool sign /fd SHA256 /sha1 $devCert.Thumbprint $outMsix
}

Write-Host "=== 6. Creating msixupload Archive for Microsoft Partner Center ===" -ForegroundColor Cyan
$tempZip = Join-Path $dist "temp_upload.zip"
$msixupload = Join-Path $dist "mesmerli.Win2PodBridge_0.1.0.0_x64.msixupload"
Remove-Item -Force $tempZip, $msixupload -ErrorAction SilentlyContinue
Compress-Archive -Path $outMsix -DestinationPath $tempZip -Force
Move-Item -Path $tempZip -Destination $msixupload -Force

# Clean up staging directory
Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue

Write-Host ""
Write-Host "==========================================================================" -ForegroundColor Green
Write-Host "Build Complete!" -ForegroundColor Green
Write-Host "  MSIX Package:     $outMsix ($([math]::Round((Get-Item $outMsix).Length/1MB,2)) MB)" -ForegroundColor Yellow
Write-Host "  MSIX Upload:      $msixupload ($([math]::Round((Get-Item $msixupload).Length/1MB,2)) MB)" -ForegroundColor Yellow
Write-Host "  Dev Certificate:  $certPath" -ForegroundColor Yellow
Write-Host "==========================================================================" -ForegroundColor Green
