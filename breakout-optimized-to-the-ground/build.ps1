$ErrorActionPreference = 'Stop'
# no_std + raw-dylib + Rust's bundled lld: no Visual Studio and no import libraries are needed.
#
# Two toolchains build this clean of any C runtime:
# - An MSVC install (`rustup target add x86_64-pc-windows-msvc`): its linker adds no CRT
#   objects by itself. `$env:RUSTFLAGS = '-C link-arg=/NODEFAULTLIB -C link-arg=/DEBUG:NONE'`
#   is the classic pong-style 15 KB link.
# - A gnullvm install (Rust + LLVM-mingw): LLVM-mingw would add crt2.o, whose own startup
#   function collides with ours, so '-C link-arg=-nostartfiles' keeps the CRT objects out.
Push-Location $PSScriptRoot
try {
    $msvcDir = (rustc --print target-libdir --target x86_64-pc-windows-msvc)
    if (Test-Path (Join-Path $msvcDir 'libstd-rs*.rlib')) {
        # MSVC std is installed: the plain build links the raw-dylib exe without any CRT.
        Remove-Item Env:RUSTFLAGS -ErrorAction SilentlyContinue
        cargo build --release --target x86_64-pc-windows-msvc
        if ($LASTEXITCODE -ne 0) { throw 'Rust build failed.' }
        $exe = 'target\x86_64-pc-windows-msvc\release\breakout-rs.exe'
    } else {
        # gnullvm host: keep LLVM-mingw's startup objects away from our entry point.
        $env:RUSTFLAGS = '-C link-arg=-nostartfiles'
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw 'Rust build failed.' }
        $exe = 'target\release\breakout-rs.exe'
    }
    Copy-Item $exe 'breakout-rs.exe' -Force
    Get-Item -LiteralPath 'breakout-rs.exe' | Select-Object FullName, Length
} finally {
    Pop-Location
}
