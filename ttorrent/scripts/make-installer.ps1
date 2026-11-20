# Copyright (C) 2026  t21 dev. GPL v2 or later.
# Builds tTorrent-<version>-windows-x64-setup.exe from a staged release folder
# (build\Release without ttorrent-old*.exe, plus COPYING.txt; see VERIFY.md › Releases).
# It writes the File and Delete lists for ttorrent\installer\ttorrent.nsi from
# that folder, so the uninstaller removes exactly what was installed.
# Needs NSIS 3 (winget install NSIS.NSIS).
# Usage: powershell -ExecutionPolicy Bypass -File ttorrent\scripts\make-installer.ps1 -Dist <staged folder> -Out <setup.exe>

param(
    [Parameter(Mandatory)][string]$Dist,
    [Parameter(Mandatory)][string]$Out
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$nsi = Join-Path $repo 'ttorrent\installer\ttorrent.nsi'
$dist = (Resolve-Path $Dist).Path
$out = [IO.Path]::GetFullPath($Out)

$version = (Select-String -Path (Join-Path $repo 'src\base\ttorrentversion.h') -Pattern '#define TTORRENT_VERSION "([^"]+)"').Matches[0].Groups[1].Value
if (-not (Test-Path (Join-Path $dist 'ttorrent.exe'))) { throw "no ttorrent.exe in $dist" }
if (-not (Test-Path (Join-Path $dist 'COPYING.txt'))) { throw "no COPYING.txt in $dist" }
if (Get-ChildItem $dist -Filter 'ttorrent-old*.exe') { throw "$dist still has ttorrent-old*.exe" }

$makensis = @("${env:ProgramFiles(x86)}\NSIS\makensis.exe", "$env:ProgramFiles\NSIS\makensis.exe") | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $makensis) { throw 'makensis.exe not found; install NSIS 3 (winget install NSIS.NSIS)' }

$work = Join-Path ([IO.Path]::GetTempPath()) "ttorrent-nsis-$PID"
New-Item -ItemType Directory -Force $work | Out-Null
try {
    $files = [Collections.Generic.List[string]]::new()
    $unfiles = [Collections.Generic.List[string]]::new()
    $dirs = Get-ChildItem $dist -Recurse -Directory | ForEach-Object { $_.FullName.Substring($dist.Length + 1) }
    foreach ($dir in @('') + $dirs) {
        $here = if ($dir) { Join-Path $dist $dir } else { $dist }
        $files.Add("SetOutPath `"`$INSTDIR$(if ($dir) { "\$dir" })`"")
        foreach ($f in Get-ChildItem $here -File) {
            $rel = $f.FullName.Substring($dist.Length + 1)
            $files.Add("File `"$($f.FullName)`"")
            $unfiles.Add("Delete `"`$INSTDIR\$rel`"")
        }
    }
    $files.Add('SetOutPath "$INSTDIR"')
    # Deepest folders first, and only if empty.
    $dirs | Sort-Object { $_.Length } -Descending | ForEach-Object { $unfiles.Add("RMDir `"`$INSTDIR\$_`"") }

    $filesNsh = Join-Path $work 'files.nsh'
    $unfilesNsh = Join-Path $work 'unfiles.nsh'
    [IO.File]::WriteAllLines($filesNsh, $files, [Text.UTF8Encoding]::new($true))
    [IO.File]::WriteAllLines($unfilesNsh, $unfiles, [Text.UTF8Encoding]::new($true))

    & $makensis /V2 "/DTT_VERSION=$version" "/DTT_DIST=$dist" "/DTT_FILES=$filesNsh" "/DTT_UNFILES=$unfilesNsh" "/DTT_OUT=$out" $nsi
    if ($LASTEXITCODE) { throw "makensis failed ($LASTEXITCODE)" }
}
finally {
    Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue
}
Write-Host "built: $out ($([math]::Round((Get-Item $out).Length / 1MB, 1)) MB, tTorrent $version)"
