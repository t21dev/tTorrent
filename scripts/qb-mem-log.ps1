# Logs qBittorrent memory every N minutes to a CSV, read-only.
# Usage: powershell -File scripts\qb-mem-log.ps1 [-Minutes 5] [-Hours 24]
param([int]$Minutes = 5, [int]$Hours = 24)
$out = Join-Path $PSScriptRoot "..\data\qb-mem.csv"
New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null
if (-not (Test-Path $out)) { "time,uptime_h,private_mb,working_set_mb,peak_ws_mb,handles,threads" | Out-File $out -Encoding utf8 }
$end = (Get-Date).AddHours($Hours)
while ((Get-Date) -lt $end) {
  $p = Get-Process qbittorrent -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($p) {
    $up = [math]::Round(((Get-Date) - $p.StartTime).TotalHours, 2)
    "{0},{1},{2},{3},{4},{5},{6}" -f (Get-Date -Format s), $up,
      [math]::Round($p.PrivateMemorySize64/1MB), [math]::Round($p.WorkingSet64/1MB),
      [math]::Round($p.PeakWorkingSet64/1MB), $p.HandleCount, $p.Threads.Count |
      Out-File $out -Append -Encoding utf8
  }
  Start-Sleep -Seconds ($Minutes * 60)
}
