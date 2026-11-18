# Installs the latest local build for everyday testing: copies build\Release
# to %LOCALAPPDATA%\Programs\tTorrent and keeps a "tTorrent" Start Menu
# shortcut pointing there. Run tTorrent from the Start Menu; builds then never
# collide with a running copy, and a restart picks up the newest version.
# If tTorrent is running, every file it holds open (the exe and its DLLs) is
# renamed aside, which Windows allows, and the new one copied in, so a new Qt
# or OpenSSL never ends up mixed with the old. Leftovers go on the next install.
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
# Files renamed aside by an earlier install, once nothing holds them any more.
Get-ChildItem $target -Recurse -File | Where-Object { $_.Name -match '\.old-\d{8}-\d{6}$' -or $_.Name -like 'ttorrent-old*.exe' } |
    ForEach-Object { try { Remove-Item $_.FullName -Force -ErrorAction Stop } catch {} }

$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$running = [bool](Get-Process ttorrent -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$target\*" })
$copied = 0
Get-ChildItem $release -Recurse -File | Where-Object { $_.Extension -notin '.pdb', '.ilk' -and $_.Name -notlike 'ttorrent-old*' } | ForEach-Object {
    $src = $_
    $dest = Join-Path $target $src.FullName.Substring($release.Length + 1)
    if ((Test-Path $dest) -and ((Get-Item $dest).Length -eq $src.Length) -and ((Get-Item $dest).LastWriteTimeUtc -eq $src.LastWriteTimeUtc)) { return }
    New-Item -ItemType Directory -Force (Split-Path $dest) | Out-Null
    try { Copy-Item $src.FullName $dest -Force -ErrorAction Stop }
    catch {
        # In use: move the old file aside and put the new one in its place.
        if (Test-Path $dest) { Rename-Item $dest ((Split-Path $dest -Leaf) + ".old-$stamp") }
        Copy-Item $src.FullName $dest -Force
    }
    $copied++
}
if ($running) { Write-Host 'tTorrent is running: restart it to use the new version.' }

$exe = Join-Path $target 'ttorrent.exe'
$shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'tTorrent.lnk'
$shell = New-Object -ComObject WScript.Shell
$link = $shell.CreateShortcut($shortcut)
$link.TargetPath = $exe
$link.WorkingDirectory = $target
# Windows caches a shortcut's icon by file path and often keeps the old one
# after the exe changes (the taskbar then shows a stale logo). The icon gets
# a name from its content, so a new logo is a new path and is always re-read.
$ico = Join-Path $repo 'ttorrent/brand/ttorrent.ico'
$hash = (Get-FileHash $ico -Algorithm SHA256).Hash.Substring(0, 8).ToLower()
Get-ChildItem $target -Filter 'ttorrent-icon-*.ico' | Where-Object Name -ne "ttorrent-icon-$hash.ico" | ForEach-Object { try { Remove-Item $_.FullName -Force -ErrorAction Stop } catch {} }
$iconFile = Join-Path $target "ttorrent-icon-$hash.ico"
if (-not (Test-Path $iconFile)) { Copy-Item $ico $iconFile }
$link.IconLocation = "$iconFile,0"
$link.Description = 'tTorrent (local build)'
$link.Save()
# Ask Explorer to re-read icons now (refreshes the taskbar and Start Menu).
Start-Process -FilePath "$env:SystemRoot\System32\ie4uinit.exe" -ArgumentList '-show' -Wait -ErrorAction SilentlyContinue
Write-Host ("installed {0} ({1}, {2} files updated)" -f $exe, (Get-Item $exe).LastWriteTime.ToString('yyyy-MM-dd HH:mm'), $copied)
