# Build environment for the restyled qBittorrent fork (Windows, MSVC 2022).
# Installs, under T:\dev: Qt 6.8 (aqtinstall) and vcpkg with libtorrent,
# Boost, OpenSSL and zlib; clones qBittorrent release-5.3.0 to
# T:\Work\t21\qbittorrent-fork. Safe to run again: finished steps are skipped.
# Usage: powershell -ExecutionPolicy Bypass -File qbittorrent\scripts\setup-build.ps1

$ErrorActionPreference = 'Stop'
$dev = 'T:\dev'
$qtVersion = '6.8.3'
New-Item -ItemType Directory -Force $dev | Out-Null

# Qt
$qt = Join-Path $dev "Qt\$qtVersion\msvc2022_64"
if (-not (Test-Path "$qt\bin\qmake.exe")) {
    Write-Host "Qt ${qtVersion}: downloading"
    aqt install-qt windows desktop $qtVersion win64_msvc2022_64 --outputdir (Join-Path $dev 'Qt')
} else { Write-Host "Qt ${qtVersion}: present" }

# vcpkg and the libraries
$vcpkg = Join-Path $dev 'vcpkg'
if (-not (Test-Path "$vcpkg\vcpkg.exe")) {
    git clone --depth 1 https://github.com/microsoft/vcpkg.git $vcpkg
    & "$vcpkg\bootstrap-vcpkg.bat" -disableMetrics
}
Write-Host "vcpkg: libtorrent, boost, openssl, zlib (x64-windows)"
& "$vcpkg\vcpkg.exe" install --triplet x64-windows libtorrent boost-circular-buffer boost-optional boost-asio openssl zlib

# Source
$src = 'T:\Work\t21\qbittorrent-fork'
if (-not (Test-Path "$src\.git")) {
    git clone --branch release-5.3.0 --depth 1 https://github.com/qbittorrent/qBittorrent.git $src
    git -C $src checkout -b dose
}
Write-Host "ready: Qt at $qt, vcpkg at $vcpkg, source at $src"
