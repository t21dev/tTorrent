# Installs the latest local build for everyday testing: copies build\Release
# to Program Files\tTorrent (where the installer puts tTorrent) and keeps a
# "tTorrent" Start Menu shortcut pointing there. Run tTorrent from the Start
# Menu; builds then never collide with a running copy, and a restart picks up
# the newest version. Program Files needs admin rights, so the copy runs in an
# elevated PowerShell: Windows asks once per run.
# If tTorrent is running, every file it holds open (the exe and its DLLs) is
# renamed aside, which Windows allows, and the new one copied in, so a new Qt
# or OpenSSL never ends up mixed with the old. Leftovers go on the next install.
# A copy in the old place (%LOCALAPPDATA%\Programs\tTorrent, 0.6.4 and earlier)
# is moved: starting with Windows, magnet and .torrent handling and the
# firewall rule are pointed at the new path, then the old folder goes.
# Torrents and settings (the profile) are never touched.
# Usage: powershell -ExecutionPolicy Bypass -File ttorrent\scripts\install-local.ps1 [-Build]

param([switch]$Build, [string]$Log)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) '..\..')).Path
$release = Join-Path $repo 'build\Release'
$target = Join-Path $env:ProgramFiles 'tTorrent'
$oldTarget = Join-Path $env:LOCALAPPDATA 'Programs\tTorrent'

if ($Build) {
    cmake --build (Join-Path $repo 'build') --config Release --parallel
    if ($LASTEXITCODE) { throw "build failed ($LASTEXITCODE)" }
}
if (-not (Test-Path (Join-Path $release 'ttorrent.exe'))) { throw "no build at $release" }

$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $admin) {
    # Run the rest elevated and show what it printed here.
    $log = Join-Path ([IO.Path]::GetTempPath()) "ttorrent-install-local-$PID.log"
    $script = $MyInvocation.MyCommand.Path
    $proc = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden `
        -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$script`"", '-Log', "`"$log`"")
    if (Test-Path $log) { Get-Content $log | Write-Host; Remove-Item $log -Force }
    exit $proc.ExitCode
}
# Write-Host isn't seen from the elevated window, so it also goes to -Log.
function Write-Note([string]$Text) { Write-Host $Text; if ($Log) { Add-Content -LiteralPath $Log -Value $Text -Encoding UTF8 } }
try {
    # The old per-user copy has to be closed before it can go.
    if ((Test-Path $oldTarget) -and (Get-Process ttorrent -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$oldTarget\*" })) {
        throw "tTorrent is running from $oldTarget. Close it (File > Exit or the tray menu) and run this again."
    }

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
    if ($running) { Write-Note 'tTorrent is running: restart it to use the new version.' }
    $exe = Join-Path $target 'ttorrent.exe'

    # Move the old per-user copy: point what referred to it at the new exe.
    $oldExe = Join-Path $oldTarget 'ttorrent.exe'
    if (Test-Path $oldExe) {
        $values = @(
            @('HKCU:\Software\Microsoft\Windows\CurrentVersion\Run', 'tTorrent'),
            @('HKCU:\Software\Classes\tTorrent.torrent\DefaultIcon', '(default)'),
            @('HKCU:\Software\Classes\tTorrent.torrent\shell\open\command', '(default)'),
            @('HKCU:\Software\Classes\tTorrent.magnet\DefaultIcon', '(default)'),
            @('HKCU:\Software\Classes\tTorrent.magnet\shell\open\command', '(default)'),
            @('HKCU:\Software\Classes\magnet\DefaultIcon', '(default)'),
            @('HKCU:\Software\Classes\magnet\shell\open\command', '(default)'),
            @('HKCU:\Software\tTorrent\Capabilities', 'ApplicationIcon'))
        foreach ($v in $values) {
            $key = Get-Item -LiteralPath $v[0] -ErrorAction SilentlyContinue
            if (-not $key) { continue }
            $name = if ($v[1] -eq '(default)') { '' } else { $v[1] }
            $old = $key.GetValue($name)
            if ($old -is [string] -and $old.IndexOf($oldExe, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
                $new = [regex]::Replace($old, [regex]::Escape($oldExe), $exe.Replace('$', '$$'), 'IgnoreCase')
                Set-ItemProperty -LiteralPath $v[0] -Name $v[1] -Value $new
            }
        }
        netsh advfirewall firewall show rule name=tTorrent | Out-Null
        if (-not $LASTEXITCODE) { netsh advfirewall firewall set rule name=tTorrent new program="$exe" | Out-Null }
        Remove-Item -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\tTorrent' -Recurse -ErrorAction SilentlyContinue
        Remove-Item (Join-Path ([Environment]::GetFolderPath('Programs')) 'tTorrent.lnk') -ErrorAction SilentlyContinue
        Remove-Item (Join-Path ([Environment]::GetFolderPath('Desktop')) 'tTorrent.lnk') -ErrorAction SilentlyContinue
        Remove-Item -Recurse -Force $oldTarget
        Write-Note "moved tTorrent from $oldTarget"
    }

    # All users' Start Menu, as the installer does.
    $shortcut = Join-Path ([Environment]::GetFolderPath('CommonPrograms')) 'tTorrent.lnk'
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
    Write-Note ("installed {0} ({1}, {2} files updated)" -f $exe, (Get-Item $exe).LastWriteTime.ToString('yyyy-MM-dd HH:mm'), $copied)
}
catch {
    Write-Note "install-local failed: $_"
    exit 1
}
