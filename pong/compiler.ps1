// COMPILER


param(
    [string]$SourceDir = $PSScriptRoot,
    [string]$OutputDir = $SourceDir
)

$ErrorActionPreference = 'Stop'
$buildDir = Join-Path ([IO.Path]::GetTempPath()) ('pong-build-' + [guid]::NewGuid().ToString('N'))
$sourceCopy = Join-Path $buildDir 'src'
$classes = Join-Path $buildDir 'classes'
$inputDir = Join-Path $buildDir 'input'
New-Item -ItemType Directory -Force -Path $sourceCopy, (Join-Path $sourceCopy 'objects'), $classes, $inputDir, $OutputDir | Out-Null
Copy-Item -LiteralPath (Join-Path $SourceDir 'Game.java'), (Join-Path $SourceDir 'Engine.java') -Destination $sourceCopy
Copy-Item -Path (Join-Path $SourceDir 'objects\*.java') -Destination (Join-Path $sourceCopy 'objects')
# Load the bundled credentials even when launched from another directory.
# An external .env in the working directory still takes priority.
$enginePath = Join-Path $sourceCopy 'Engine.java'
$engine = [IO.File]::ReadAllText($enginePath)
$old = 'List<String> lines = Files.readAllLines(Path.of(fileName));'
$replacement = @'
List<String> lines;
            if (Files.isRegularFile(Path.of(fileName))) {
                lines = Files.readAllLines(Path.of(fileName));
            } else {
                try (var stream = Engine.class.getResourceAsStream("/" + fileName)) {
                    if (stream == null) {
                        throw new java.io.IOException("No bundled configuration found");
                    }
                    lines = new String(stream.readAllBytes(), java.nio.charset.StandardCharsets.UTF_8).lines().toList();
                }
            }
'@
if (-not $engine.Contains($old)) { throw 'Engine.java changed: configuration loader needs review.' }
[IO.File]::WriteAllText($enginePath, $engine.Replace($old, $replacement))
$sources = @(Get-ChildItem -LiteralPath $sourceCopy -Filter '*.java' -Recurse | ForEach-Object FullName)
& javac --release 17 -encoding UTF-8 -d $classes @sources
if ($LASTEXITCODE -ne 0) { throw 'Java compilation failed.' }
Copy-Item -LiteralPath (Join-Path $SourceDir '.env') -Destination (Join-Path $classes '.env')
& jar "-J-Djava.io.tmpdir=$buildDir" --create --file (Join-Path $inputDir 'Pongie.jar') --main-class Game -C $classes .
if ($LASTEXITCODE -ne 0) { throw 'JAR creation failed.' }
$launcherCode = @'
using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Security.Cryptography;
using System.Threading;
using System.Windows.Forms;

class StandalonePong
{
    [STAThread]
    static int Main()
    {
        try
        {
            string version;
            using (Stream data = Assembly.GetExecutingAssembly().GetManifestResourceStream("PongPayload"))
            using (SHA256 sha = SHA256.Create())
                version = BitConverter.ToString(sha.ComputeHash(data)).Replace("-", "").Substring(0, 16);
            string cache = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "Pongie", version);
            using (Mutex gate = new Mutex(false, "Local\\PongieExtract" + version))
            {
                try { gate.WaitOne(); } catch (AbandonedMutexException) { }
                try
                {
                    string marker = Path.Combine(cache, "ready");
                    if (!File.Exists(marker))
                    {
                        Directory.CreateDirectory(cache);
                        using (Stream data = Assembly.GetExecutingAssembly().GetManifestResourceStream("PongPayload"))
                        using (ZipArchive zip = new ZipArchive(data, ZipArchiveMode.Read))
                        {
                            foreach (ZipArchiveEntry entry in zip.Entries)
                            {
                                string path = Path.GetFullPath(Path.Combine(cache, entry.FullName));
                                if (!path.StartsWith(cache + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                                    throw new IOException("Invalid package path.");
                                if (String.IsNullOrEmpty(entry.Name)) { Directory.CreateDirectory(path); continue; }
                                Directory.CreateDirectory(Path.GetDirectoryName(path));
                                entry.ExtractToFile(path, true);
                            }
                        }
                        File.WriteAllText(marker, version);
                    }
                }
                finally { gate.ReleaseMutex(); }
            }
            ProcessStartInfo start = new ProcessStartInfo(Path.Combine(cache, "Pongie", "runtime", "bin", "javaw.exe"));
            start.WorkingDirectory = Path.Combine(cache, "Pongie");
            start.UseShellExecute = false;
            // Short socket paths avoid Java networking failures with long TEMP paths.
            string socketDir = Path.GetDirectoryName(Assembly.GetExecutingAssembly().Location);
            start.Arguments = "-Djdk.net.unixdomain.tmpdir=\"" + socketDir + "\" -jar app\\Pongie.jar";
            using (Process game = Process.Start(start)) { game.WaitForExit(); return game.ExitCode; }
        }
        catch (Exception error)
        {
            MessageBox.Show(error.Message, "Pong could not start", MessageBoxButtons.OK, MessageBoxIcon.Error);
            return 1;
        }
    }
}

'@
$payloadRoot = Join-Path $buildDir 'payload'
$appRoot = Join-Path $payloadRoot 'Pongie'
New-Item -ItemType Directory -Force -Path (Join-Path $appRoot 'app') | Out-Null
Copy-Item -LiteralPath (Join-Path $inputDir 'Pongie.jar') -Destination (Join-Path $appRoot 'app\Pongie.jar')
$jdkBin = Split-Path (Get-Command javac.exe -ErrorAction Stop).Source -Parent
& (Join-Path $jdkBin 'jlink.exe') --add-modules java.desktop,java.net.http,jdk.crypto.mscapi,jdk.crypto.ec --strip-debug --no-header-files --no-man-pages --output (Join-Path $appRoot 'runtime')
if ($LASTEXITCODE -ne 0) { throw 'Bundled runtime creation failed.' }
$payloadZip = Join-Path $buildDir 'payload.zip'
Compress-Archive -LiteralPath $appRoot -DestinationPath $payloadZip -CompressionLevel Optimal
$launcherPath = Join-Path $buildDir 'StandalonePong.cs'
[IO.File]::WriteAllText($launcherPath, $launcherCode)
$csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
$builtExe = Join-Path $buildDir 'pong.exe'
& $csc /nologo /target:winexe /platform:x64 "/out:$builtExe" /reference:System.Windows.Forms.dll /reference:System.IO.Compression.dll /reference:System.IO.Compression.FileSystem.dll "/resource:$payloadZip,PongPayload" $launcherPath
if ($LASTEXITCODE -ne 0) { throw 'Standalone EXE compilation failed.' }
Copy-Item -LiteralPath $builtExe -Destination (Join-Path $OutputDir 'pong.exe') -Force
Copy-Item -LiteralPath (Join-Path $inputDir 'Pongie.jar') -Destination (Join-Path $OutputDir 'pong.jar') -Force
Get-Item -LiteralPath (Join-Path $OutputDir 'pong.exe'), (Join-Path $OutputDir 'pong.jar') | Select-Object FullName,Length,LastWriteTime
# Remove only this build's unique temporary directory.
$resolvedBuild = [IO.Path]::GetFullPath($buildDir)
$resolvedTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\')
if ((Split-Path $resolvedBuild -Parent) -eq $resolvedTemp -and (Split-Path $resolvedBuild -Leaf) -like 'pong-build-*') {
    Remove-Item -LiteralPath $resolvedBuild -Recurse -Force
}