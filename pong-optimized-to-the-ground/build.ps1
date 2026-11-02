$ErrorActionPreference = 'Stop'
# Static CRT so the exe doesn't need libunwind.dll from the toolchain.
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --release --manifest-path (Join-Path $PSScriptRoot 'Cargo.toml')
if ($LASTEXITCODE -ne 0) {
    throw 'Rust build failed.'
}

$output = Join-Path $PSScriptRoot 'pong-rs.exe'
Copy-Item (Join-Path $PSScriptRoot 'target\release\pong-rs.exe') $output -Force
Get-Item -LiteralPath $output | Select-Object FullName, Length
