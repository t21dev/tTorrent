# Build environment for tTorrent on Windows (MSVC Build Tools 2022, CMake).
# Installs Qt 6.12 (ttorrent\scripts\install-qt.py; aqtinstall can't read the
# 6.12 repository layout yet) and vcpkg under -Dev (default L:\dev). The
# libraries come from ttorrent\vcpkg\vcpkg.json when build.ps1 configures.
# Safe to run again: finished steps are skipped.
# Usage: powershell -ExecutionPolicy Bypass -File ttorrent\scripts\setup.ps1

param([string]$Dev = 'L:\dev')
$ErrorActionPreference = 'Stop'
$qtVersion = '6.12.1'
New-Item -ItemType Directory -Force $Dev | Out-Null

$qt = Join-Path $Dev "Qt\$qtVersion\msvc2022_64"
if (-not (Test-Path "$qt\bin\Qt6Core.dll")) {
    Write-Host "Qt ${qtVersion}: downloading"
    python -m pip install -q py7zr
    python -I (Join-Path $PSScriptRoot 'install-qt.py') $qtVersion (Join-Path $Dev 'Qt')
} else { Write-Host "Qt ${qtVersion}: present" }

$vcpkg = Join-Path $Dev 'vcpkg'
if (-not (Test-Path "$vcpkg\vcpkg.exe")) {
    # Full history: the manifest's baseline and version overrides need it.
    git clone https://github.com/microsoft/vcpkg.git $vcpkg
    & "$vcpkg\bootstrap-vcpkg.bat" -disableMetrics
}
Write-Host "ready: Qt at $qt, vcpkg at $vcpkg (libraries install on the first build)"
