# Configure and build tTorrent on Windows (after setup.ps1).
# Libraries come from ttorrent\vcpkg\vcpkg.json (OpenSSL pinned to the 3.5 LTS
# line) and are built into <build>\vcpkg_installed the first time.
# Output: <build>\Release\ttorrent.exe with its Qt and library DLLs beside it.
# Usage: powershell -ExecutionPolicy Bypass -File ttorrent\scripts\build.ps1 [-Clean] [-Build build] [-QtVersion 6.12.1]

param([switch]$Clean, [string]$Dev = 'L:\dev', [string]$Build = 'build', [string]$QtVersion = '6.12.1')
$ErrorActionPreference = 'Stop'
$src = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$build = Join-Path $src $Build
$qt = Join-Path $Dev "Qt\$QtVersion\msvc2022_64"
$vcpkg = Join-Path $Dev 'vcpkg'

if ($Clean -and (Test-Path $build)) { Remove-Item -Recurse -Force $build }

cmake -S $src -B $build -G 'Visual Studio 17 2022' -A x64 `
    "-DCMAKE_TOOLCHAIN_FILE=$vcpkg\scripts\buildsystems\vcpkg.cmake" `
    -DVCPKG_TARGET_TRIPLET=x64-windows `
    "-DVCPKG_MANIFEST_DIR=$src\ttorrent\vcpkg" `
    "-DCMAKE_PREFIX_PATH=$qt" `
    -DGUI=ON -DWEBUI=ON -DPLUGINS=OFF -DSTACKTRACE=OFF -DTESTING=OFF
if ($LASTEXITCODE) { throw "configure failed ($LASTEXITCODE)" }

cmake --build $build --config Release --parallel
if ($LASTEXITCODE) { throw "build failed ($LASTEXITCODE)" }

$exe = Join-Path $build 'Release\ttorrent.exe'
if (-not (Test-Path $exe)) { throw 'ttorrent.exe not found after the build' }
& "$qt\bin\windeployqt.exe" --release --no-translations --no-system-d3d-compiler --no-opengl-sw $exe | Out-Null
Write-Host "built: $exe"
