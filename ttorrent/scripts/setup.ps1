# Build environment for tTorrent on Windows (MSVC Build Tools 2022, CMake).
# Installs Qt 6.8 (aqtinstall) and vcpkg with libtorrent, Boost, OpenSSL and
# zlib under -Dev (default T:\dev). Safe to run again: finished steps are skipped.
# Usage: powershell -ExecutionPolicy Bypass -File ttorrent\scripts\setup.ps1

param([string]$Dev = 'T:\dev')
$ErrorActionPreference = 'Stop'
$qtVersion = '6.8.3'
New-Item -ItemType Directory -Force $Dev | Out-Null

$qt = Join-Path $Dev "Qt\$qtVersion\msvc2022_64"
if (-not (Test-Path "$qt\bin\qmake.exe")) {
    Write-Host "Qt ${qtVersion}: downloading"
    aqt install-qt windows desktop $qtVersion win64_msvc2022_64 --outputdir (Join-Path $Dev 'Qt')
} else { Write-Host "Qt ${qtVersion}: present" }

$vcpkg = Join-Path $Dev 'vcpkg'
if (-not (Test-Path "$vcpkg\vcpkg.exe")) {
    git clone --depth 1 https://github.com/microsoft/vcpkg.git $vcpkg
    & "$vcpkg\bootstrap-vcpkg.bat" -disableMetrics
}
Write-Host "vcpkg: libtorrent, boost, openssl, zlib (x64-windows)"
& "$vcpkg\vcpkg.exe" install --triplet x64-windows libtorrent boost-circular-buffer boost-optional boost-asio openssl zlib
Write-Host "ready: Qt at $qt, vcpkg at $vcpkg"
