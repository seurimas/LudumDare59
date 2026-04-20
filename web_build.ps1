$ErrorActionPreference = 'Stop'

$dist = "dist"

# Ensure wasm target is installed
Write-Host "Ensuring wasm32-unknown-unknown target is installed..."
rustup target add wasm32-unknown-unknown

# Build for wasm (override crate-type to cdylib for wasm-bindgen)
Write-Host "Building for wasm32-unknown-unknown (release)..."
cargo rustc --lib --release --target wasm32-unknown-unknown --crate-type cdylib
if ($LASTEXITCODE -ne 0) { throw "Cargo build failed" }

# Run wasm-bindgen to generate JS glue
Write-Host "Running wasm-bindgen..."
wasm-bindgen --out-dir $dist --target web "target\wasm32-unknown-unknown\release\LudumDare59.wasm"
if ($LASTEXITCODE -ne 0) { throw "wasm-bindgen failed. Install it with: cargo install wasm-bindgen-cli" }

# Copy assets
Write-Host "Copying assets..."
if (Test-Path "$dist\assets") { Remove-Item "$dist\assets" -Recurse -Force }
Copy-Item "assets" "$dist\assets" -Recurse

# Copy index.html
Copy-Item "web\index.html" "$dist\index.html"

Write-Host "Web build complete in $dist/"
Write-Host "Serve it with: python -m http.server -d $dist"
