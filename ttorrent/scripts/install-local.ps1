# Installs the latest local build for everyday testing: copies build\Release
# to %LOCALAPPDATA%\Programs\tTorrent and keeps a "tTorrent" Start Menu
# shortcut pointing there. Run tTorrent from the Start Menu; builds then never
# collide with a running copy, and a restart picks up the newest version.
# If tTorrent is running, its exe is renamed aside (Windows allows that) and
# the new one copied in; the old copy is removed on the next install.
# Usage: powershell -ExecutionPolicy Bypass -File ttorrent\scripts\install-local.ps1 [-Build]

param([switch]$Build)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) '..\..')).Path
$release = Join-Path $repo 'build\Release'
$target = Join-Path $env:LOCALAPPDATA 'Programs\tTorrent'

if ($Build) {
    cmake --build (Join-Path $repo 'build') --config Release --parallel
    if ($LASTEXITCODE) { throw "build failed ($LASTEXITCODE)" }
}
if (-not (Test-Path (Join-Path $release 'ttorrent.exe'))) { throw "no build at $release" }

New-Item -ItemType Directory -Force $target | Out-Null
Get-ChildItem $target -Filter 'ttorrent-old*.exe' -ErrorAction SilentlyContinue | ForEach-Object { try { Remove-Item $_.FullName -Force -ErrorAction Stop } catch {} }
$exe = Join-Path $target 'ttorrent.exe'
if ((Test-Path $exe) -and (Get-Process ttorrent -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe })) {
    Rename-Item $exe ('ttorrent-old-{0}.exe' -f (Get-Date -Format 'yyyyMMdd-HHmmss'))
    Write-Host 'tTorrent is running: restart it to use the new version.'
}
# Copy everything except stale exes; unchanged DLLs in use are skipped.
robocopy $release $target /E /XF 'ttorrent-old*.exe' '*.pdb' '*.ilk' /R:1 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) { throw "copy failed (robocopy $LASTEXITCODE)" }

$shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'tTorrent.lnk'
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($shortcut)
$link.TargetPath = $exe
$link.WorkingDirectory = $target
$link.IconLocation = "$exe,0"
$link.Description = 'tTorrent (local build)'
$link.Save()
Write-Host ("installed {0} ({1})" -f $exe, (Get-Item $exe).LastWriteTime.ToString('yyyy-MM-dd HH:mm'))
