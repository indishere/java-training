# COMPILER

$ErrorActionPreference = 'Stop'
$classes = Join-Path ([IO.Path]::GetTempPath()) ('breakout-build-' + [guid]::NewGuid().ToString('N'))
& javac --release 21 -encoding UTF-8 -d $classes (Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.java' -Recurse | ForEach-Object FullName)
if ($LASTEXITCODE -ne 0) { throw 'Java compilation failed.' }
& jar --create --file (Join-Path $PSScriptRoot 'breakout.jar') --main-class Game -C $classes .
if ($LASTEXITCODE -ne 0) { throw 'JAR creation failed.' }
Remove-Item -LiteralPath $classes -Recurse -Force
