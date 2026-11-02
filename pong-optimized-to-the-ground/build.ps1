$ErrorActionPreference = 'Stop'
$compiler = Get-Command clang++.exe -ErrorAction SilentlyContinue
if (-not $compiler) {
    $compiler = Get-Command g++.exe -ErrorAction SilentlyContinue
}
if (-not $compiler) {
    throw 'Install LLVM-MinGW or MinGW-w64 and put clang++ or g++ on PATH.'
}

$output = Join-Path $PSScriptRoot 'pong-cpp.exe'
& $compiler.Source -std=c++17 -O2 -DNDEBUG -mwindows -static -s `
    (Join-Path $PSScriptRoot 'pong.cpp') -o $output -lgdi32 -luser32
if ($LASTEXITCODE -ne 0) {
    throw 'C++ build failed.'
}

Get-Item -LiteralPath $output | Select-Object FullName, Length
