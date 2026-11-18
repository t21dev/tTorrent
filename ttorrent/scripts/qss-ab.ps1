# Which stylesheet groups cost memory: start-up runs of one tTorrent build with
# the full stylesheet, with none, and with one group of rules dropped at a time
# (TTORRENT_BENCH_QSS_DROP, see ttorrent/VERIFY.md › Benchmarks).
# Usage: powershell -ExecutionPolicy Bypass -File ttorrent\scripts\qss-ab.ps1 -Exe <snapshot>\ttorrent.exe -Source <profile copy> -Out <folder>

param([Parameter(Mandatory)][string]$Exe, [Parameter(Mandatory)][string]$Source, [Parameter(Mandatory)][string]$Out, [int]$Runs = 2, [int]$Seconds = 100)
$ErrorActionPreference = 'Stop'
$bench = Join-Path $PSScriptRoot 'bench.ps1'
$variants = [ordered]@{
    'full'      = ''
    'none'      = '*'
    'menus'     = 'QMenu'
    'scroll'    = 'QScrollBar'
    'headtabs'  = 'QHeaderView,QTabBar,QTabWidget'
    'inputs'    = 'QLineEdit,QComboBox,QSpinBox,QDoubleSpinBox,QDateTimeEdit,QPlainTextEdit,QTextEdit'
    'checks'    = 'QCheckBox,QRadioButton'
    'groups'    = 'QGroupBox'
    'views'     = 'QTreeView,QTableView,QListView,QAbstractItemView'
}
foreach ($name in $variants.Keys) {
    Remove-Item Env:TTORRENT_BENCH_NO_QSS, Env:TTORRENT_BENCH_QSS_DROP -ErrorAction SilentlyContinue
    if ($variants[$name] -eq '*') { $env:TTORRENT_BENCH_NO_QSS = '1' }
    elseif ($variants[$name]) { $env:TTORRENT_BENCH_QSS_DROP = $variants[$name] }
    & powershell -ExecutionPolicy Bypass -File $bench -Source $Source -Out (Join-Path $Out $name) -TtExe $Exe -Apps tt -StartupRuns $Runs -StartupSeconds $Seconds -SoakMinutes 0 | Out-Null
}
Remove-Item Env:TTORRENT_BENCH_NO_QSS, Env:TTORRENT_BENCH_QSS_DROP -ErrorAction SilentlyContinue
Write-Host "done: $Out"
