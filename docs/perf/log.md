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
