# Measures qBittorrent's disk I/O and memory during startup, read-only.
# Usage: quit qBittorrent, run this script, then start qBittorrent.
#   powershell -File scripts\qb-startup-io.ps1 [-Seconds 120]
# Writes data\qb-startup-io.csv (one row per second) and prints a summary.
param([int]$Seconds = 120, [string]$Name = "qbittorrent")
$out = Join-Path $PSScriptRoot "..\data\qb-startup-io.csv"
New-Item -ItemType Directory -Force (Split-Path $out) | Out-Null
Write-Host "Waiting for $Name to start..."
$p = $null
while (-not $p) { $p = Get-CimInstance Win32_Process -Filter "Name='$Name.exe'" -ErrorAction SilentlyContinue | Select-Object -First 1; if (-not $p) { Start-Sleep -Milliseconds 200 } }
$id = $p.ProcessId; $t0 = Get-Date
Write-Host "Found PID $id. Sampling for $Seconds s..."
"t_s,read_mb,write_mb,read_ops,write_ops,other_mb,private_mb,working_set_mb,handles,threads" | Out-File $out -Encoding utf8
$first = $null; $last = $null
for ($i = 0; $i -le $Seconds; $i++) {
  $c = Get-CimInstance Win32_Process -Filter "ProcessId=$id" -ErrorAction SilentlyContinue
  if (-not $c) { Write-Host "Process exited."; break }
  if (-not $first) { $first = $c }
  $last = $c
  $row = "{0},{1},{2},{3},{4},{5},{6},{7},{8},{9}" -f [math]::Round(((Get-Date) - $t0).TotalSeconds, 1),
    [math]::Round($c.ReadTransferCount / 1MB, 1), [math]::Round($c.WriteTransferCount / 1MB, 1),
    $c.ReadOperationCount, $c.WriteOperationCount, [math]::Round($c.OtherTransferCount / 1MB, 1),
    [math]::Round($c.PrivatePageCount / 1MB), [math]::Round($c.WorkingSetSize / 1MB), $c.HandleCount, $c.ThreadCount
  $row | Out-File $out -Append -Encoding utf8
  Start-Sleep -Seconds 1
}
if ($last) {
  Write-Host ("Read {0} MB in {1} ops, wrote {2} MB, peak handles {3}. CSV: {4}" -f [math]::Round($last.ReadTransferCount / 1MB, 1), $last.ReadOperationCount, [math]::Round($last.WriteTransferCount / 1MB, 1), $last.HandleCount, (Resolve-Path $out))
}
