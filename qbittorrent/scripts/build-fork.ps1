# Configure and build the restyled qBittorrent fork (after setup-build.ps1).
# Output: T:\Work\t21\qbittorrent-fork\build\Release\qbittorrent.exe with its
# Qt and library DLLs beside it (windeployqt, vcpkg app-local copies).
# Usage: powershell -ExecutionPolicy Bypass -File qbittorrent\scripts\build-fork.ps1 [-Clean]

param([switch]$Clean)
$ErrorActionPreference = 'Stop'
$src = 'T:\Work\t21\qbittorrent-fork'
$build = Join-Path $src 'build'
$qt = 'T:\dev\Qt\6.8.3\msvc2022_64'
$vcpkg = 'T:\dev\vcpkg'

if ($Clean -and (Test-Path $build)) { Remove-Item -Recurse -Force $build }

cmake -S $src -B $build -G 'Visual Studio 17 2022' -A x64 `
    "-DCMAKE_TOOLCHAIN_FILE=$vcpkg\scripts\buildsystems\vcpkg.cmake" `
    -DVCPKG_TARGET_TRIPLET=x64-windows `
    "-DCMAKE_PREFIX_PATH=$qt" `
    -DGUI=ON -DWEBUI=ON -DPLUGINS=OFF -DSTACKTRACE=OFF -DTESTING=OFF
if ($LASTEXITCODE) { throw "configure failed ($LASTEXITCODE)" }

cmake --build $build --config Release --parallel
if ($LASTEXITCODE) { throw "build failed ($LASTEXITCODE)" }

$exe = Get-ChildItem -Path $build -Recurse -Filter ttorrent.exe | Where-Object { $_.FullName -match '\\Release\\' } | Select-Object -First 1
if (-not $exe) { throw 'ttorrent.exe not found after the build' }
& "$qt\bin\windeployqt.exe" --release --no-translations --no-system-d3d-compiler $exe.FullName | Out-Null
Write-Host "built: $($exe.FullName)"
