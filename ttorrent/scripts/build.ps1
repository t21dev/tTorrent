# Configure and build tTorrent on Windows (after setup.ps1).
# Output: build\Release\ttorrent.exe with its Qt and library DLLs beside it.
# Usage: powershell -ExecutionPolicy Bypass -File ttorrent\scripts\build.ps1 [-Clean]

param([switch]$Clean, [string]$Dev = 'T:\dev')
$ErrorActionPreference = 'Stop'
$src = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$build = Join-Path $src 'build'
$qt = Join-Path $Dev 'Qt\6.8.3\msvc2022_64'
$vcpkg = Join-Path $Dev 'vcpkg'

if ($Clean -and (Test-Path $build)) { Remove-Item -Recurse -Force $build }

cmake -S $src -B $build -G 'Visual Studio 17 2022' -A x64 `
    "-DCMAKE_TOOLCHAIN_FILE=$vcpkg\scripts\buildsystems\vcpkg.cmake" `
    -DVCPKG_TARGET_TRIPLET=x64-windows `
    "-DCMAKE_PREFIX_PATH=$qt" `
    -DGUI=ON -DWEBUI=ON -DPLUGINS=OFF -DSTACKTRACE=OFF -DTESTING=OFF
if ($LASTEXITCODE) { throw "configure failed ($LASTEXITCODE)" }

cmake --build $build --config Release --parallel
if ($LASTEXITCODE) { throw "build failed ($LASTEXITCODE)" }

$exe = Join-Path $build 'Release\ttorrent.exe'
if (-not (Test-Path $exe)) { throw 'ttorrent.exe not found after the build' }
& "$qt\bin\windeployqt.exe" --release --no-translations --no-system-d3d-compiler --no-opengl-sw $exe | Out-Null
Write-Host "built: $exe"
