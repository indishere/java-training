$ErrorActionPreference = 'Stop'
# no_std + raw-dylib + Rust's bundled lld: needs only `rustup target add x86_64-pc-windows-msvc`,
# no Visual Studio or MinGW. Run from this folder so cargo picks up .cargo/config.toml.
Push-Location $PSScriptRoot
try {
    cargo build --release --target x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) {
        throw 'Rust build failed.'
    }
    Copy-Item 'target\x86_64-pc-windows-msvc\release\pong-rs.exe' 'pong-rs.exe' -Force
    Get-Item -LiteralPath 'pong-rs.exe' | Select-Object FullName, Length
} finally {
    Pop-Location
}
