# Performance guideline

How we keep tTorrent fast and small while it grows: what we measure, how we compare a change against `main`, when a change is blocked, and what to try when a change makes things slower or doesn't help.

Budgets live in `PLAN.md` §1. Correctness and layout tests are in `TESTING.md`; the done-checklist is `VERIFY.md`. Results and experiments go in `docs/perf/log.md`. The method follows Kitab (`../kitab/docs/perf/baseline.md`, `kitab-bench`).

## 1. Principles

1. **Measure, never guess.** A number without its machine, date, build, scenario and method doesn't count.
2. **Budgets are hard limits.** A change that breaks a budget doesn't merge, however good it is otherwise.
3. **Compare like with like.** Same machine, same fixture, A and B runs interleaved, medians plus p95, noise measured first.
4. **Count what is deterministic.** Bytes read, files opened, allocations, rechecks and peak heap don't depend on machine noise. They run everywhere, including CI. Wall-clock time and frame times run only on the reference machines.
5. **Every attempt is written down**, kept or rejected, so nobody repeats a dead end.
6. **No feature cuts.** "Faster because it does less" is not a fix (see `AGENTS.md`).

## 2. What we measure

| ID | Metric | Scenario | Budget (PLAN.md) | Regression threshold vs `main` |
|---|---|---|---|---|
| **M1** | Task Manager memory and private bytes, engine + window | 735 torrents, window open, 10 min idle | 100 MB target, 120 MB hard (Task Manager; private bytes and GPU memory recorded) | +2 MB or +3 %, whichever is larger |
| **M2** | Same, window closed (tray only) | 735 torrents, 10 min | < 30 MB, 50 MB hard | +1 MB or +3 % |
| **M3** | Memory slope | 24 h soak, seeding | flat (day 7 within 10 % of day 1) | any positive trend above noise |
| **M4** | Idle CPU | 10 min, window open and closed | 0 %, 2 % hard | +0.2 % points |
| **T1** | Launch to list on screen | warm and cold start, 735 torrents | < 300 ms, 500 ms hard | +5 % or +10 ms |
| **T2** | Launch to all torrents running | 735 torrents | < 60 s | +10 % |
| **D1** | Disk read in the first 60 s, excluding pieces peers asked for | 735 torrents, no files changed | < 50 MB, 200 MB hard | +10 % or +5 MB |
| **D2** | Full hash checks at startup with no file changed | same | 0 | any |
| **D3** | Peak open file handles | same | ≤ file-pool size | any increase |
| **D4** | Disk written in the first 60 s | same | < 5 MB | +10 % |
| **N1** | Download throughput, CPU per MB | local swarm, 4 GB, 20 peers | equal to qBittorrent | −3 % speed or +5 % CPU/MB |
| **N2** | Upload throughput | local swarm, 40 leechers | equal to qBittorrent | −3 % |
| **U1** | List scroll frame time p50 / p95 / p99 | 735 and 5,000 torrents, fling scroll | p95 < 8 ms, p99 < 16.7 ms | +10 % p95 |
| **U2** | Sort, filter keystroke, tab switch, menu open, settings search keystroke (input to frame) | 735 and 5,000 torrents | < 50 ms each, < 16.7 ms target | +10 % or +5 ms |
| **U3** | Animation frame time during transitions (toolbar item, tab underline, accent change) | 1080p and 4K window | p95 < 16.7 ms | +10 % p95 |
| **S1** | Scaling: memory and T1 at 100, 735, 5,000 torrents | synthetic fixtures | per-torrent cost flat or falling | per-torrent cost +5 % |
| **B1** | Installer and on-disk size | release build | installer < 15 MB (set at spike) | +5 % |
| **A1** | Allocations and peak heap per scenario | B1 to U2 scenarios, counted | — | +5 % (deterministic, runs in CI) |

Memory is always read three ways, as in Kitab: Task Manager (private working set), working set and private bytes, plus GPU memory if a GPU stack is used.

## 3. The suite: `tt-bench`

A Rust crate, black box like `kitab-bench`: it starts the real release binary, drives it from outside, and reads OS counters. Windows first, macOS and Linux later.

### Fixtures (never the owner's data)
- **Synthetic libraries** of 100, 735 and 5,000 torrents, generated from a fixed seed: same file-count and size distribution as a real library (median 2 files, 2 MiB pieces, about 11 trackers each), with **sparse files** on disk so a 1.5 TB library costs almost nothing. Resume data is written as both tTorrent's SQLite and qBittorrent `BT_backup`, so the same fixture benchmarks both apps and the importer.
- **Local swarm**: a local tracker plus seeder and leecher instances on loopback serving generated data, so N1 and N2 are repeatable and offline.
- **Changed-file cases** for D2: one file truncated, one touched, one deleted.

### Commands

```
cargo xtask perf quick  [--against main]   # Tier 1, ~3 min
cargo xtask perf full   [--against main]   # Tier 2, ~20 min
cargo xtask perf nightly                   # Tier 3: soak, 5,000 torrents, cold starts, iGPU machine
cargo xtask perf compare A.json B.json     # report only
cargo xtask perf qbittorrent               # same scenarios against qBittorrent, for the comparison table
```

Each run writes `docs/perf/runs/<date>-<commit>.json` (not committed, except baselines) and prints a Markdown table ready to paste into a commit, PR or `log.md`.

### Tiers

| Tier | When | Runs | Where |
|---|---|---|---|
| **0: counters** | Every push, in CI | A1, D2, D3, D1 bytes, rechecks, file opens (deterministic counts) | GitHub Actions, Windows |
| **1: quick** | Before committing any change to the engine, storage, list, rendering or startup | M1, M2, T1 warm, D1 to D4, U1 and U2 at 735, N1 small (1 GB) | Reference machine |
| **2: full** | Before merging to `main`, and for every perf experiment | All except M3, at 735 and 5,000; cold and warm starts | Reference machine |
| **3: nightly / release** | Nightly while active, and before every release | Tier 2 + M3 24 h soak + S1 + iGPU machine + qBittorrent comparison | Both reference machines |

Reference machines: **R1**, the owner's desktop (Ryzen 9 9900X, RTX 5070 and the AMD integrated GPU), and **R2**, a laptop with an integrated GPU and, if possible, a hard disk (to add).

## 4. How a comparison works

1. **Build both** in release mode: the candidate and `main` (cached by commit).
2. **Quiet the machine:** High performance power plan, no other heavy apps, the same display scale, the AC adapter on for laptops. The report records all of this.
3. **Measure the noise first:** `main` against itself, 15 runs. The coefficient of variation per metric is the noise floor.
4. **Interleave:** A B A B …, 15 runs each after 2 warm-ups, so drift (thermals, background tasks) hits both equally.
5. **Decide per metric:**
   - **Improved**: median better by more than the threshold, and the bootstrap 95 % interval of the difference excludes zero.
   - **Regressed**: median worse by more than `max(threshold, 2 × noise)`, and the interval excludes zero.
   - **Unchanged**: anything else.
6. **Report:** one row per metric with the baseline, candidate, difference, interval and verdict. Budgets are checked on the candidate's p95, not only its median.

## 5. Gates

| Result | What happens |
|---|---|
| Any budget broken | **Blocked.** Go to §6. |
| A regression on any metric | **Blocked** unless the owner accepts a written waiver: which metric, by how much, what it buys, and a follow-up to win it back. The waiver goes in `log.md`. |
| A change meant to speed something up shows "unchanged" | **Not merged as a perf change.** Keep it only if it's simpler or needed for something else; otherwise go to §6 or drop it. |
| Improved, nothing regressed | Merge; record the numbers in `log.md`. If it beats a baseline, update the baseline file in the same change. |

## 6. When a change is slower, or not faster: the alternatives ladder

Never "try harder at random". Work down this list, time-boxed (about half a day per rung), and write each attempt in `log.md` with its numbers, even when it fails.

**Step 1. Confirm.** Re-run interleaved. Check the noise floor. If the difference is within the noise, it isn't real; stop.

**Step 2. Locate.** Find where the time, memory or I/O goes before changing anything:
- trace marks (`tracing` spans around startup phases, waves, first frame, sort, filter)
- CPU: `samply` or Windows Performance Recorder
- memory: `dhat` (heap) and private bytes over time; on Windows, VMMap for non-heap memory (driver, GPU, mapped)
- disk: the I/O counters `tt-bench` records, Process Monitor or ETW file I/O for the pattern (sequential vs random, which files)
- frames: the UI frame-time log

**Step 3. Climb the ladder**, cheapest first:

| Rung | Idea | Examples for tTorrent |
|---|---|---|
| 1. Do the same work cheaper | Fewer allocations and copies, reuse buffers, batch, avoid locks on hot paths | Reuse row strings, batch SQLite writes in one transaction, intern tracker URLs |
| 2. Do it later | Defer until needed, after first paint, at background priority | Load details on selection, start torrents in waves, open files on demand |
| 3. Do it less often | Coalesce, debounce, redraw only what changed | Update speeds once a second, redraw only dirty rows, one write per pause |
| 4. Store it differently | Compact layouts, IDs instead of strings, bitsets, struct-of-arrays, data on disk instead of RAM | 20-byte peer records, piece hashes on disk for finished torrents |
| 5. Flip cache and recompute | Add a bounded cache (capped in MB) or remove one that costs more than it saves | Glyph cache cap, list snapshot vs live query |
| 6. Move it | Engine vs window, another thread, the OS (page cache, background I/O) | Hash checks off the UI path, sorting in SQL |
| 7. Change the tool | Another algorithm, crate, allocator, SQLite setting or renderer path | mimalloc, a SIMD SHA-1, SQLite WAL and page size, CPU vs GPU renderer |
| 8. Change the design, same result for the user | Same outcome, different mechanism; needs the owner's yes | Show the snapshot first instead of waiting for live data |
| 9. Make the cost opt-in | Only for a genuinely optional extra that some users want; never to hide a regression in the default path, and never by lowering a default limit | A heavier visual effect behind a setting |

**Step 4. Decide.** Bring the owner a table: each option tried, its numbers on every affected metric, its complexity, and a recommendation. If nothing on the ladder meets the budget, the options are to change the design (rung 8), defer the feature, or change the budget. That is the owner's call, never an agent's.

## 7. Rules for agents and contributors

- Changes to the engine, storage, startup, the list, rendering or the window run **Tier 1** before committing and paste the comparison table in the commit message or PR.
- Perf experiments run **Tier 2** and get a `log.md` entry, kept or not.
- A new feature arrives with its own scenario in `tt-bench` if it touches a hot path (e.g. RSS refresh with 200 feeds, MCP `list_torrents` with 5,000 torrents).
- Never compare numbers from different machines, fixtures or methods in the same table.
- Baselines are updated only by a merged change that improved them, with the run file committed under `docs/perf/runs/baseline-*.json`.
