# COMPILER

$ErrorActionPreference = 'Stop'
$build = Join-Path ([IO.Path]::GetTempPath()) ('breakout-build-' + [guid]::NewGuid().ToString('N'))
$classes = Join-Path $build 'classes'
$app = Join-Path $build 'payload\BreakOut'
$jar = Join-Path $PSScriptRoot 'breakout.jar'

& javac --release 21 -encoding UTF-8 -d $classes (Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.java' -Recurse | ForEach-Object FullName)
if ($LASTEXITCODE -ne 0) { throw 'Java compilation failed.' }
& jar --create --file $jar --main-class Game -C $classes .
if ($LASTEXITCODE -ne 0) { throw 'JAR creation failed.' }

# Bundle the jar with a trimmed Java runtime so the exe runs without Java installed
New-Item -ItemType Directory -Force -Path $app | Out-Null
Copy-Item -LiteralPath $jar -Destination (Join-Path $app 'breakout.jar')
$jdkBin = Split-Path (Get-Command javac.exe).Source -Parent
& (Join-Path $jdkBin 'jlink.exe') --add-modules java.desktop --strip-debug --no-header-files --no-man-pages --output (Join-Path $app 'runtime')
if ($LASTEXITCODE -ne 0) { throw 'Runtime creation failed.' }
$payload = Join-Path $build 'payload.zip'
Compress-Archive -LiteralPath $app -DestinationPath $payload -CompressionLevel Optimal

# Launcher: unpacks the payload once into %LOCALAPPDATA%\BreakOut\<hash>, then starts javaw
$launcher = Join-Path $build 'Launcher.cs'
[IO.File]::WriteAllText($launcher, @'
using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Security.Cryptography;
using System.Threading;
using System.Windows.Forms;

class Launcher
{
    [STAThread]
    static int Main()
    {
        try
        {
            Assembly self = Assembly.GetExecutingAssembly();
            string version;
            using (Stream data = self.GetManifestResourceStream("Payload"))
            using (SHA256 sha = SHA256.Create())
                version = BitConverter.ToString(sha.ComputeHash(data)).Replace("-", "").Substring(0, 16);
            string cache = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "BreakOut", version);
            using (Mutex gate = new Mutex(false, "Local\\BreakOutExtract" + version))
            {
                try { gate.WaitOne(); } catch (AbandonedMutexException) { }
                try
                {
                    string marker = Path.Combine(cache, "ready");
                    if (!File.Exists(marker))
                    {
                        if (Directory.Exists(cache)) Directory.Delete(cache, true);
                        using (Stream data = self.GetManifestResourceStream("Payload"))
                        using (ZipArchive zip = new ZipArchive(data, ZipArchiveMode.Read))
                            zip.ExtractToDirectory(cache);
                        File.WriteAllText(marker, version);
                    }
                }
                finally { gate.ReleaseMutex(); }
            }
            string home = Path.Combine(cache, "BreakOut");
            ProcessStartInfo start = new ProcessStartInfo(Path.Combine(home, "runtime", "bin", "javaw.exe"), "-jar breakout.jar");
            start.WorkingDirectory = home;
            start.UseShellExecute = false;
            using (Process game = Process.Start(start)) { game.WaitForExit(); return game.ExitCode; }
        }
        catch (Exception error)
        {
            MessageBox.Show(error.Message, "BreakOut could not start", MessageBoxButtons.OK, MessageBoxIcon.Error);
            return 1;
        }
    }
}
'@)
$csc = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319\csc.exe'
& $csc /nologo /target:winexe /platform:x64 "/out:$(Join-Path $PSScriptRoot 'breakout.exe')" /reference:System.Windows.Forms.dll /reference:System.IO.Compression.dll /reference:System.IO.Compression.FileSystem.dll "/resource:$payload,Payload" $launcher
if ($LASTEXITCODE -ne 0) { throw 'EXE compilation failed.' }

Remove-Item -LiteralPath $build -Recurse -Force
