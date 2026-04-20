$ErrorActionPreference = 'Stop'

Write-Host "Building release binary..."
cargo build --bin LudumDare59 --release
if ($LASTEXITCODE -ne 0) { throw "Cargo build failed" }

$zip = "RunicAscendancy_LD59.zip"
if (Test-Path $zip) { Remove-Item $zip }

$staging = "build_staging"
if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
New-Item $staging -ItemType Directory | Out-Null

Copy-Item "target\release\LudumDare59.exe" "$staging\LudumDare59.exe"
Copy-Item "assets" "$staging\assets" -Recurse

Compress-Archive -Path "$staging\*" -DestinationPath $zip
Remove-Item $staging -Recurse -Force

Write-Host "Created $zip"
