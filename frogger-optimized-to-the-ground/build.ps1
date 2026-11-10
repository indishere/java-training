$ErrorActionPreference = 'Continue' # cargo writes progress notes to stderr; PS 5.1 would treat that as a crash under 'Stop'
# Produces one frogger-rs.exe with no runtime dependency beyond Windows' own
# DLLs (raw-dylib imports; links with the toolchain's own lld: no Visual Studio,
# no MinGW install needed beyond Rust itself). PNGs load from assets\ next to
# the exe, so the game folder carries them along like the Java sketch does.
Push-Location $PSScriptRoot
try {
    cargo build --release
    $ErrorActionPreference = 'Stop'
    if ($LASTEXITCODE -ne 0) {
        throw 'Rust build failed.'
    }
    Copy-Item 'target\release\frogger-rs.exe' 'frogger-rs.exe' -Force
    Get-Item -LiteralPath 'frogger-rs.exe' | Select-Object FullName, Length
} finally {
    Pop-Location
}
