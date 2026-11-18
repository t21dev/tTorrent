# Performance log

Every measurement and experiment, newest first, kept or rejected. Method and gates: `docs/perf/README.md`.

Entry format:

```
## <date> · <short title> · <kept | rejected | waived>
Machine: R1 (Ryzen 9 9900X, RTX 5070 + AMD iGPU), build <commit>, fixture <library-735>, tier <1|2|3>
Goal: what we tried to improve, and why
Tried: the rung(s) from the alternatives ladder
| Metric | Baseline | Candidate | Δ | 95 % interval | Verdict |
Decision: kept / rejected / waived, and why. Follow-up if any.
```

## 2026-11-18 · librqbit fork: trust fastresume when file sizes match · kept

Machine: R1, release builds, `tt-bench startup` with memory sampled once every torrent has started (see the correction below), two runs each.
Goal: D1, no disk spike at start. Locate: librqbit's `validate_fastresume` hashes one piece of every file plus a sample of up to 64, for every torrent on every start.
Tried: rung 4, a size check instead of reading: `SessionOptions::fastresume_check = FileSizes` trusts the saved bitfield when every file with pieces is at exactly its expected length, and falls back to sampling otherwise. Branch `fastresume-file-sizes` of t21dev/rqbit, upstream PR ikatson/rqbit#758; tTorrent uses branch `ttorrent` (both fixes) and sets `FileSizes`.

| 735 torrents | Sample (librqbit default) | FileSizes | Δ |
|---|---|---|---|
| Reads to all started, 3.5 TB | 6,373 MB | 27–28 MB | −99.6 % |
| Reads to all started, capped at 64 MB | 2,090 MB | 2.9 MB | −99.9 % |
| CPU time, first 60 s, 3.5 TB | 4.91 s | 1.98 s | −60 % |
| Launch to list, 3.5 TB | 453–563 ms | 413–427 ms | |

The remaining 27 MB is close to the stored .torrent files' piece hashes (1.26 M × 20 B = 25 MB); not traced by file. librqbit's new e2e test shows 0 reads on restore with FileSizes and a fall-back check after a file is truncated.
Steady state with both fixes, 3.5 TB, every torrent started: window open 116.6–117.3 MB, tray only 88.2–88.5 MB (M1 over target and under hard; M2 broken). Capped library: 79.8 / 51.5 MB.
Decision: kept.

## 2026-11-18 · correction: `tt-bench startup` sampled memory while torrents were still starting

`tt-bench startup` took its memory samples about 10 and 20 s after the list appeared, while startup waves were still bringing torrents online (all 735 take about 45 s). Fewer running torrents means less memory, so those figures were flattering: "window open 89.0 MB, tray only 67.6 MB" (sparse chunk status entry) and the tTorrent column of the qBittorrent comparison (55.9 / 35.9 MB). It now waits until every torrent has started and samples 5 s later; the figures above and the README use that. Found when the faster start (FileSizes) moved the timeline: the same build measured 68 MB tray-only before the change and 88 MB after it. The list detector also missed rows drawn in the "Starting" state (62 % opacity); its threshold now counts them.

## 2026-11-18 · librqbit fork: sparse chunk status · kept

Machine: R1, release builds, fixture `tt-bench library 735` (3.5 TB, 1.26 M pieces), `tt-bench engine-mem`, before and after run back to back. Single runs.
Goal: M2. Locate: `ChunkTracker::chunk_status` in librqbit holds one bit per 16 KiB chunk of every torrent, finished or not: about 27 MB here, half of the 45 B per piece found earlier (the other half is the 20-byte piece hashes, which librqbit already shares zero-copy with the .torrent bytes).
Tried: rung 4 (data on demand): per-chunk state only for pieces in flight, a map from piece to its chunk bits, dropped when the piece passes or fails its hash check. Branch `sparse-chunk-status` of t21dev/rqbit, upstream PR ikatson/rqbit#757; tTorrent uses it through `[patch.crates-io]`.

| Engine only, Task Manager | Before (crates.io 9.1.0) | After (fork) | Δ |
|---|---|---|---|
| 3 s after start | 77.9 MB | 51.3 MB | −26.6 MB |
| 63 s, all live | 112.1 MB | 84.5 MB | −27.6 MB |
| all stopped, 20 s later | 81.6 MB | 54.2 MB | −27.4 MB |
| capped library (43 GB), 63 s | 49.4 MB (earlier run) | 48.6 MB | none beyond noise (0.3 MB of chunk bits) |

Full app with the fork (`tt-bench startup`): window open 89.0 MB (M1 under its 100 MB target), tray only 67.6 MB (M2 still over 50 MB hard), launch to list 563 ms in this run.
Decision: kept. Tests: librqbit 50 passed (e2e download and stream included), tTorrent 76 passed. Next for M2: the 20-byte piece hashes of finished torrents (on disk, read on demand).

## 2026-11-18 · qBittorrent on the same library · measured

Machine: R1. `tt-bench qb` runs the installed qBittorrent 5.3.0 (libtorrent 2.1.2, Qt 6.11.2) from an isolated `--profile` (its own settings, ports, resume data), offline (no DHT, LSD, PeX, UPnP, peer-country lookup or update check), torrents added through its Web API with `skip_checking`, then two measured launches (window, then hidden in the tray). tTorrent measured with `tt-bench startup` on the same fixture minutes before, crates.io librqbit 9.1.0. Single runs; both fresh starts, so qBittorrent's growth with uptime (about 1.3 GB on the owner's machine) is not in these numbers.
Fixture: `tt-bench library 735 .tt-dev/lib735-64 --max-mb 64` (735 torrents, 43 GB, 88 k pieces). On the full 3.5 TB library qBittorrent ignored `skip_checking` for 516 of 735 torrents and queued full checks, which hadn't finished after 10 minutes; that run is still to do.

| Measure | tTorrent | qBittorrent 5.3.0 |
|---|---|---|
| Launch to window / list | 463 ms (list painted) | 165 ms (window shown), 256 ms to all torrents loaded |
| Task Manager, window open | 55.9 MB (engine 29.7 + window 26.2) | 32.2 MB |
| Task Manager, tray only | 35.9 MB | 23.2 MB |
| File reads, first 60 s | 2,090 MB | 2.9 MB |
| File writes, first 60 s | 200 MB | 0.1 MB |
| CPU time, first 60 s | 3.02 s | 0.42 s |
| Idle CPU, 30 s | 1.04 % of one core | 0.11 % (window), 0.00 % (tray) |

Finding: on a fresh start qBittorrent is ahead on every measure here. The disk numbers point at librqbit's fast-resume sampling and `session.json` rewrites (PLAN.md change list); the idle CPU at tTorrent's 1 Hz tick over every torrent. These are the next targets.

## 2026-11-18 · startup disk and idle CPU, 735-torrent synthetic library · measured

Machine: R1, release build of the working tree (uncommitted), fixture `tt-bench library 735` (as below), engine `--offline`. Single run. A qBittorrent 5.3.0 process was running on the machine at the same time (not ours; it may have competed for disk and CPU).
Method: `tt-bench startup .tt-dev/lib735`, extended with `tt_platform::process_io` (`GetProcessIoCounters` and `GetProcessTimes` of the engine process). Reads count bytes the process asked for, including what the OS served from its file cache, so they are an upper bound on physical disk reads; on this sparse fixture most reads are of holes and cost almost nothing.

| Metric | Value |
|---|---|
| T1 launch to list on screen | 453 ms |
| Engine file reads, launch to 60 s (every wave done) | 6,450 MB in 112,708 reads |
| Engine file writes, same | 219.5 MB |
| Engine CPU time, same | 5.28 s |
| Engine CPU idle in the tray, 30 s | 2.45 % of one core, 0.10 % of all 24 logical CPUs (Task Manager's scale) |
| Window CPU, first 10 s after the list showed | 2.96 % of one core |
| Task Manager, window open (engine + window) | 116.3 MB |
| Task Manager, tray only | 96.9 MB |

Finding: startup reads come from librqbit's fast-resume check (a sample of pieces in every file, `validate_fastresume`), the item already on the librqbit change list. The 219.5 MB of writes fit `session.json` (about 300 KB for 735 torrents) being rewritten each time a torrent starts or pauses; not yet proven with a file-level trace. Idle CPU has no budget yet; 2.45 % of one core with nothing transferring is worth a profile (the 1 Hz tick walks every torrent).
Decision: recorded. Follow-up: a file-level trace (ETW or Process Monitor) to split reads and writes by file, then the two librqbit items.

## 2026-11-18 · first build: 735-torrent synthetic library · measured, budgets broken

Machine: R1 (Ryzen 9 9900X, RTX 5070 + AMD iGPU; GPUI on its default adapter, not pinned), release build of the first working tree (uncommitted), fixture `tt-bench library 735` (735 single-file torrents, 1,257,434 pieces, 3.5 TB sparse zero-filled content, librqbit resume data so nothing is hashed), engine started with `--offline` (no DHT, trackers, LSD, UPnP). Tier: single runs, **not** the 15-run interleaved method of §4, so treat these as a first locate, not a baseline.
Goal: first check of M1, M2 and T1 against PLAN.md §1.
Method: `tt-bench startup .tt-dev/lib735` (Task Manager = `PROCESS_MEMORY_COUNTERS_EX2.PrivateWorkingSetSize`, the same counter Task Manager shows), then `tt-bench engine-mem`. Cross-checked once with PowerShell `Get-Process` (working set and private bytes agreed).

| Metric | Budget | Run 1 (no snapshot) | Run 2 (warm) | Verdict |
|---|---|---|---|---|
| M1 Task Manager, engine + window, window open | 100 MB target, 120 hard | 114.0 MB | 117.6 MB | over target, under hard |
| of which window (`ttorrent-ui`) | — | 26.8 MB | 30.0 MB | |
| of which engine | — | 87.2 MB | 87.6 MB | |
| M2 Task Manager, tray only (engine) | 30 MB target, 50 hard | 95.4 MB | 95.9 MB | **broken** |
| Private bytes, window open | flag above 200 MB | 188.2 MB | 191.7 MB | under the flag |
| T1 launch to list on screen | 300 ms target, 500 hard | 498 ms | 472 ms | over target, at the hard edge |

Locate (`tt-bench engine-mem`, engine only, Task Manager):

| Library (735 torrents) | Pieces | 3 s after start | 63 s, all live | all stopped, 20 s |
|---|---|---|---|---|
| sizes capped at 64 MB (`--max-mb 64`) | 88,330 | 25.0 MB | 49.4 MB | 26.3 MB |
| full sizes | 1,257,434 | 77.7 MB | 112.2 MB | 86.2 MB |

Finding: engine memory follows the **piece count**, about 45 bytes per piece while loaded (a 20-byte SHA-1 held about twice, plus bitfields), plus about 33 KB per live torrent. The owner's library has 1.1 M pieces, so it lands in the second row. The window process is small (27–30 MB).
Decision: recorded, nothing waived. M2 needs the planned storage change (rung 4): piece hashes on disk for finished torrents and one copy of metadata in memory. Both are inside librqbit (`TorrentMetadata` keeps `info`, `torrent_bytes` and `info_bytes`), so they go on the librqbit change list in PLAN.md. Budget changes are the owner's call; none was made.

## 2026-11-18 · demo library, 14 torrents, window memory · measured

Machine: R1, release build, `tt-bench demo` (14 synthetic open-content torrents, 3 downloading at about 9 MB/s from a loopback seeder, 7 seeding), window 1280 × 800.

| State | Engine TM | Window TM | Both TM | Both private bytes |
|---|---|---|---|---|
| Main list, light | 5.2 MB | 29.3 MB | 34.6 MB | 102.2 MB |
| Settings sheet open, dark | 5.3 MB | 34.1 MB | 39.4 MB | 144.6 MB |
| Window closed (engine only) | 5.2 MB | – | 5.2 MB | 6.5 MB |

Notes: single runs. Private bytes of the window vary between 101 and 145 MB across the six shots with the same content; not investigated yet (GPU driver allocations are the likely cause, see Kitab's baseline).

## 2026-11-18 · hash check stalled half way · fixed (bug, not perf)

Found with `tt-bench demo`: a seeded torrent stayed at 56 % "Checking" forever. Cause: the download queue counted torrents that were still checking as "downloading" and paused the fifth one mid-check. Fix: only downloading, stalled, metadata and queued torrents take part in the queue (qBittorrent's rule), with a regression test (`checking_torrents_are_not_queued`). Raising tokio's blocking pool (`TT_BLOCKING=128`) was tried first and made no difference; rejected.

## 2026-11-17 · qBittorrent baseline, owner's library · measured

Machine: R1, qBittorrent 5.2.4 (libtorrent build not yet confirmed), the owner's 735 torrents (1.57 TB, 2,063 files, 1.1 M pieces, 270 private), not a synthetic fixture.

| Measure | Value | How |
|---|---|---|
| Private bytes, under 1 h after start | 214 to 221 MB | `Get-Process`, PrivateMemorySize64 |
| Working set, same | 264 to 281 MB | `Get-Process`, WorkingSet64 |
| Task Manager memory after long uptime | ~1,322 MB | Owner's Task Manager screenshot |
| Memory over 24 h | in progress | `scripts/qb-mem-log.ps1` → `data/qb-mem.csv` |
| Startup disk I/O | not yet measured | `scripts/qb-startup-io.ps1` |

Notes: memory grows with uptime and falls back after a restart. Likely causes are listed in `PLAN.md` §1. This is the number tTorrent is compared against; the synthetic 735-torrent fixture will be checked against it so later comparisons can use the fixture.
