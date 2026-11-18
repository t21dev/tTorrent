# Startup and memory comparison: stock qBittorrent against tTorrent, each run
# on a fresh copy of the same profile. Sequential runs only: two clients on
# the same files would fight over them.
#
# Each run copies -Source (a profile copy with Roaming\ and Local\ folders) into
# a scratch profile, turns the Web UI off and limits downloads to 1 KiB/s (so
# the owner's files are read for seeding but barely written), starts the app
# with --profile, samples it every -Interval seconds for -Seconds, then stops it.
#
# Per sample: private working set (Task Manager's "Memory"), working set,
# private bytes, CPU %. Per run: time to the main window, time until CPU stays
# under 2 % for 5 s ("settled"), memory at the end, bytes read and written.
#
# Usage (from the repo root):
#   powershell -ExecutionPolicy Bypass -File ttorrent\scripts\bench.ps1 `
#     -Source T:\dev\ttorrent-bench\qb-profile-2026-11-18 -Out T:\dev\ttorrent-bench\results `
#     -StartupRuns 3 -StartupSeconds 120 -SoakMinutes 60

param(
    [Parameter(Mandatory)][string]$Source,
    [Parameter(Mandatory)][string]$Out,
    [string]$QbExe = "$env:ProgramFiles\qBittorrent\qbittorrent.exe",
    [string]$TtExe = '',
    [int]$StartupRuns = 3,
    [int]$StartupSeconds = 120,
    [int]$SoakMinutes = 60
)
$ErrorActionPreference = 'Stop'
if (-not $TtExe) { $TtExe = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) '..\..\build\Release\ttorrent.exe' }
New-Item -ItemType Directory -Force $Out | Out-Null
$cores = [Environment]::ProcessorCount

function Set-IniValue([string]$Path, [string]$Section, [string]$Key, [string]$Value) {
    $lines = [System.Collections.Generic.List[string]](Get-Content -LiteralPath $Path -Encoding UTF8)
    $start = $lines.IndexOf("[$Section]")
    if ($start -lt 0) { $lines.Add(''); $lines.Add("[$Section]"); $lines.Add("$Key=$Value") }
    else {
        $end = $start + 1
        while ($end -lt $lines.Count -and -not $lines[$end].StartsWith('[')) { $end++ }
        $found = $false
        for ($i = $start + 1; $i -lt $end; $i++) { if ($lines[$i].StartsWith("$Key=")) { $lines[$i] = "$Key=$Value"; $found = $true } }
        if (-not $found) { $lines.Insert($start + 1, "$Key=$Value") }
    }
    Set-Content -LiteralPath $Path -Value $lines -Encoding UTF8
}

function New-Profile([string]$App, [string]$Dir) {
    if (Test-Path $Dir) { Remove-Item -Recurse -Force $Dir }
    $config = Join-Path $Dir "$App\config"; $data = Join-Path $Dir "$App\data"
    New-Item -ItemType Directory -Force $config, $data | Out-Null
    Copy-Item -Recurse -Force (Join-Path $Source 'Roaming\*') $config
    Copy-Item -Recurse -Force (Join-Path $Source 'Local\*') $data
    if ($App -ne 'qBittorrent') {
        Rename-Item (Join-Path $config 'qBittorrent.ini') "$App.ini"
        if (Test-Path (Join-Path $config 'qBittorrent-data.ini')) { Rename-Item (Join-Path $config 'qBittorrent-data.ini') "$App-data.ini" }
    }
    $ini = Join-Path $config "$App.ini"
    Set-IniValue $ini 'LegalNotice' 'Accepted' 'true'
    Set-IniValue $ini 'Preferences' 'WebUI\Enabled' 'false'
    Set-IniValue $ini 'Preferences' 'General\PreventFromSuspendWhenDownloading' 'false'
    Set-IniValue $ini 'Application' 'GUI\Notifications\Enabled' 'false'
    Set-IniValue $ini 'BitTorrent' 'Session\GlobalDLSpeedLimit' '1'
    Set-IniValue $ini 'BitTorrent' 'Session\AlternativeGlobalDLSpeedLimit' '1'
}

function Invoke-Run([string]$Label, [string]$App, [string]$Exe, [int]$Seconds, [int]$Interval) {
    $dir = Join-Path $Out "profile-$Label"
    New-Profile $App $dir
    $csv = Join-Path $Out "$Label.csv"
    'seconds,private_ws_mb,working_set_mb,private_bytes_mb,cpu_pct' | Set-Content $csv
    $t0 = [Diagnostics.Stopwatch]::StartNew()
    $p = Start-Process -FilePath $Exe -ArgumentList "--profile=$dir" -PassThru
    $name = $p.ProcessName
    $counter = New-Object Diagnostics.PerformanceCounter('Process', 'Working Set - Private', $name, $true)
    $window = $null; $settled = $null; $quiet = 0; $lastCpu = $p.TotalProcessorTime; $lastT = $t0.Elapsed
    $next = $Interval
    while ($t0.Elapsed.TotalSeconds -lt $Seconds -and -not $p.HasExited) {
        Start-Sleep -Milliseconds 250
        $p.Refresh()
        if (-not $window -and $p.MainWindowHandle -ne 0) { $window = [math]::Round($t0.Elapsed.TotalSeconds, 2) }
        if ($t0.Elapsed.TotalSeconds -ge $next) {
            $now = $t0.Elapsed; $cpu = ($p.TotalProcessorTime - $lastCpu).TotalSeconds / ($now - $lastT).TotalSeconds / $cores * 100
            $lastCpu = $p.TotalProcessorTime; $lastT = $now
            $pws = $counter.NextValue() / 1MB
            '{0:F0},{1:F1},{2:F1},{3:F1},{4:F2}' -f $now.TotalSeconds, $pws, ($p.WorkingSet64 / 1MB), ($p.PrivateMemorySize64 / 1MB), $cpu | Add-Content $csv
            if ($window -and -not $settled) { if ($cpu -lt 2) { $quiet += $Interval; if ($quiet -ge 5) { $settled = [math]::Round($now.TotalSeconds - $quiet, 0) } } else { $quiet = 0 } }
            $next += $Interval
        }
    }
    $io = Get-CimInstance Win32_Process -Filter "ProcessId=$($p.Id)" -ErrorAction SilentlyContinue
    $p.Refresh()
    $result = [ordered]@{
        label = $Label; app = $App; seconds = $Seconds; window_s = $window; settled_s = $settled
        private_ws_mb = [math]::Round($counter.NextValue() / 1MB, 1); working_set_mb = [math]::Round($p.WorkingSet64 / 1MB, 1)
        private_bytes_mb = [math]::Round($p.PrivateMemorySize64 / 1MB, 1)
        read_mb = if ($io) { [math]::Round($io.ReadTransferCount / 1MB, 1) } else { $null }
        written_mb = if ($io) { [math]::Round($io.WriteTransferCount / 1MB, 1) } else { $null }
    }
    if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force; Start-Sleep 3 }
    Remove-Item -Recurse -Force $dir -ErrorAction SilentlyContinue
    $result | ConvertTo-Json -Compress | Add-Content (Join-Path $Out 'summary.jsonl')
    Write-Host ($result | ConvertTo-Json -Compress)
}

if (Get-Process qbittorrent, ttorrent -ErrorAction SilentlyContinue) { throw 'qBittorrent or tTorrent is running; close it first.' }
$machine = "{0}, {1:N0} GB, {2}" -f (Get-CimInstance Win32_Processor).Name.Trim(), ((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB), (Get-CimInstance Win32_OperatingSystem).Caption
"# {0} · {1} · qBittorrent {2} vs tTorrent build {3}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm'), $machine, (Get-Item $QbExe).VersionInfo.ProductVersion, (Get-Item $TtExe).LastWriteTime.ToString('yyyy-MM-dd HH:mm') | Add-Content (Join-Path $Out 'summary.jsonl')

for ($i = 1; $i -le $StartupRuns; $i++) {
    Invoke-Run "startup-qb-$i" 'qBittorrent' $QbExe $StartupSeconds 1
    Invoke-Run "startup-tt-$i" 'tTorrent' $TtExe $StartupSeconds 1
}
if ($SoakMinutes -gt 0) {
    Invoke-Run 'soak-qb' 'qBittorrent' $QbExe ($SoakMinutes * 60) 15
    Invoke-Run 'soak-tt' 'tTorrent' $TtExe ($SoakMinutes * 60) 15
}
Write-Host "done: $Out"
