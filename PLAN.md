# tTorrent plan

A torrent client with qBittorrent's features and layout, a modern finish, and memory that stays flat. UI: **GPUI** (decided 18 November 2026). Windows first, then macOS and Linux.

Living references:
- Feature picks (v1 / Later / Skip): https://claude.ai/artifact/8rWBDxNJoz7gp5RDLCGwPo (105 v1, 18 later, 9 skipped = 93.2 % of qBittorrent 5.2, plus our additions)
- Design: `DESIGN.md`, mockup `docs/design-directions-2.html`

## 1. Goals and budgets

Every memory figure is **everything tTorrent runs** (engine and window processes together), with 735 torrents. The gated number is **Task Manager memory** (private working set), the one users see; private bytes, working set and GPU memory are recorded next to it on every run.

| Budget | Target | Hard limit | Measured how |
|---|---|---|---|
| Window open, 735 torrents | as close to 50 MB as we can get; **100 MB** | **120 MB** | Task Manager, after 24 h, on the NVIDIA dGPU **and** the AMD iGPU (the iGPU is the hard case) |
| Window closed (tray only: engine process alone, no GPU) | **under 30 MB** | 50 MB | Same |
| Private bytes, window open | recorded, no gate | flag if above 200 MB on the iGPU | VMMap / `tt-bench` |
| Over time | flat: day 7 equals day 1 within 10 % | no growth trend | `scripts/qb-mem-log.ps1`-style log |
| Idle CPU (seeding, window closed or open and idle) | 0 % | 1 to 2 % | Process Explorer, 10 min |
| Cold start to list on screen | under 300 ms | 500 ms | External timing, as in Kitab |
| Startup disk reads, 735 torrents, first 60 s (excluding pieces peers asked for) | under 50 MB | 200 MB | `scripts/qb-startup-io.ps1`; see `docs/startup.md` |
| Full hash checks at startup when no file changed | 0 | 0 | Engine log |
| Throughput | equal to qBittorrent on the same torrent | within 5 % | Same swarm, same machine |

**Baseline to beat** (this machine, 17 November 2026): qBittorrent 5.2.4 with 735 torrents uses ~214 MB private right after start and grows to ~1.3 GB with uptime. Its likely causes: libtorrent 2.x memory-mapped files, peer lists of up to ~3,000 per torrent, heap fragmentation, everything loaded at start. A 24-hour log is in `data/qb-mem.csv` (not committed).

**Rule:** no feature cuts for memory. Every limit (peer list size, connections, cache, checking) is a setting with qBittorrent's default. Savings come from how data is stored: compact peer records, pooled buffers with hard MB caps, piece hashes on disk for finished torrents, lazy loading, no memory-mapped I/O.

## 2. Architecture

### Decision: GPUI (18 November 2026)

The owner chose **GPUI**: the team already ships Kitab on it (with its patches and lean mode), it gives the best look and motion for the Dose design, and it runs on Windows, macOS and Linux.

What that costs, measured in Kitab (`../kitab/docs/perf/baseline.md`, 7 November 2026), and how the design contains it:

| Kitab, lean mode | Task Manager | Private bytes | GPU memory |
|---|---|---|---|
| Kitab, 2 pages, NVIDIA RTX 5070 | 28.8 MB | 107 MB | ~70 MB |
| Kitab, 2 pages, AMD integrated GPU | 50.8 MB | 186 MB | shared with RAM |
| GPUI alone, NVIDIA / AMD iGPU | | 87 / 149 MB | |

Most of that is the Direct3D device, the graphics driver and window-sized surfaces, paid whenever a GPUI window exists. So:

```
 tt-engine (always running, no GPU)            tt-ui (GPUI, only while the window is open)
 ├─ librqbit session: all torrents             ├─ list, details, settings, menus
 ├─ SQLite: torrents, settings, RSS, stats     └─ talks to the engine over local IPC
 ├─ tray / menu-bar icon and menu                 (named pipe / Unix socket, typed API)
 ├─ Web API, MCP server (opt-in)
 └─ RSS, watched folders, scheduler, file association, single instance
```

- **Closing to the tray ends the window process**, which frees the GPU device, driver memory and surfaces. The tray-only state is the engine alone, with no GPU (budget under 30 MB).
- **Opening the window** starts `tt-ui` against a warm engine; it draws the list from the engine's snapshot, so it appears fast (`docs/startup.md`).
- **Lean mode on by default** (Kitab's single-sampled path texture and two swap-chain buffers), the font and adapter start-up fixes, and a "hardware acceleration off" fallback on WARP for machines where the GPU path misbehaves.
- **Risk:** on integrated GPUs the GPUI floor alone is near the window-open budget. The spike measures it first; if it breaks 120 MB in Task Manager there, the ladder in `docs/perf/README.md` applies (lean settings, smaller surfaces, adapter choice), and any budget change is the owner's call.

Alternatives considered before the decision (for the record): Tauri (measured ~220 to 270 MB in Turbo Reader), egui (same GPU floor as GPUI), iced with CPU rendering (lighter on paper, not measured, new to the team), Slint's software renderer (western-script text only), native per OS (three codebases).

### Engine

- **librqbit** (Rust, Apache-2.0). Already has DHT, magnets, trackers, sequential download, file selection, UPnP and an HTTP API. uTP is experimental in 9.0.
- Work we expect to add or upstream: protocol encryption, IP filter, interface binding, BitTorrent v2, compact peer storage, hashes-on-disk for finished torrents, move-storage. Confirm each in the spike.
- **SQLite** (rusqlite, bundled) for everything persistent, as in Turbo Reader.
- **Import from qBittorrent**: read `BT_backup` (`.torrent` + `.fastresume`), categories, tags and settings, keep progress without rechecking.

### Crates

```
crates/
  tt-engine     session, storage, limits, scheduler, RSS, watched folders
  tt-ipc        typed API shared by the UI, CLI and Web API
  tt-webapi     qBittorrent-compatible Web API (Sonarr, Radarr, Prowlarr, phone apps)
  tt-mcp        built-in MCP server for AI agents (docs/mcp.md), off by default
  tt-ui         the GPUI window process (started when shown, exits when closed to the tray)
  tt-theme      tokens, themes, density, scale, accent maths (DESIGN.md)
  tt-platform   tray, file association, autostart, taskbar progress, notifications
  tt-import     qBittorrent importer
  tt-bench      memory, CPU and start-up measurement
```

## 2026-11-18 · Direction change: restyle qBittorrent

A like-for-like benchmark (`tt-bench qb`, docs/perf/log.md) showed stock qBittorrent 5.3.0 lighter than tTorrent on a fresh start: memory (32 vs 80 MB window open, 23 vs 52 MB tray), idle CPU and writes. tTorrent's separate GPUI window alone costs about what all of qBittorrent does, so the window-open budget can't beat it by design. The owner chose to restyle and optimise qBittorrent instead:

- Fork: `T:\Work\t21\qbittorrent-fork`, branch `dose`, from `release-5.3.0` (GPL-3.0 binaries; local only until the owner says otherwise).
- Built in: the Dose theme as the default (`src/gui/dose/theme`, generated by `qbittorrent/build_theme.py`), a custom title bar keeping native window behaviour (`src/gui/titlebar`, QWindowKit, Apache-2.0), and the direction A Settings and Add torrent layouts (`src/gui/dose`), applied in code after `setupUi()` so upstream merges stay small.
- Build: `qbittorrent/scripts/setup-build.ps1` (Qt 6.8.3, vcpkg) and `build-fork.ps1`.
- Parked, not deleted: the Rust crates below. The librqbit PRs (ikatson/rqbit#757, #758) stand on their own.

## 3. Phases

Progress as of 18 November 2026: `[x]` done and tested, `[~]` partly done (what's missing is named), `[ ]` not started. Measurements: `docs/perf/log.md`.

### Phase 0: spike (1 to 2 weeks)
- [ ] librqbit headless: load a **copy** of the 735-torrent `BT_backup`, seed for 24 h, log memory every 5 min alongside qBittorrent's log. Needs the owner's copy; the synthetic 735-torrent fixture stands in until then.
- [~] Measure: private bytes, Task Manager memory, startup time and threads on the synthetic fixture (`tt-bench startup`, `engine-mem`). Not yet: idle CPU, handles, 15-run medians.
- [ ] Baseline qBittorrent's startup I/O with `scripts/qb-startup-io.ps1`, then the same for tTorrent.
- [ ] Check each "verify" tag in the feature map against librqbit's source (encryption, IP filter, v2, web seeds, PEX, LSD, move storage, per-torrent limits).
- [~] A GPUI window over IPC with a virtual list, sidebar, details and piece map: built and measured on the default adapter. Not yet: Kitab's lean patches, the AMD iGPU, GPU memory.
- [~] Budgets checked once (log entry of 18 November): window-open 114–118 MB (over target, under hard), **tray-only 96 MB (broken)**. librqbit changes listed below.

### Phase 1: engine
- [~] Session, SQLite state (WAL, incremental writes), librqbit fast-resume, global and slow-mode limits, pause for a time. Torrent queueing as in qBittorrent (off by default; maximum active downloads, uploads and torrents with ∞; "Do not count slow torrents" with its rate thresholds and inactivity timer; force start outside the limits). Seeding limits: ratio, total seeding time, inactive seeding time, any or all, then stop, remove, or remove with files. Not yet: per-torrent speed and seeding limits at run time (librqbit has no call), super seeding, scheduler.
- [ ] Hard memory caps (buffers, cache, peers) as settings. Memory (engine + window) is shown in the status bar.
- [~] Startup: snapshot first, torrents restored paused and started in waves (downloading first), `Start torrents gradually` setting. Not yet: size-and-mtime trust instead of librqbit's sampled piece checks, per-disk I/O queue, background I/O priority.
- [~] qBittorrent import (`ttorrent --import-qbittorrent [BT_backup]`): torrents, save paths, categories, tags, history, and the piece map written as librqbit fast-resume data. Not yet: the import sheet, settings, a run on a real library.
- [~] Missing content: torrents gone from the session show as errors, daily database backups (7 kept), integrity check at start. Not yet: the rest of `docs/missing-files.md`.
- [~] Single instance (a second launch hands over its files and links), file association on Windows, macOS and Linux with Settings › File association and `--register` (`docs/associations.md`; macOS and Linux type-checked, not run), tray with the qBittorrent menu plus live header and "Pause for 1 hour", window started on demand and ended when closed. Not yet: autostart switch in Settings (the code exists in `tt-platform::assoc`).

### Phase 2: window
- [~] List from the stored snapshot, "Starting" rows, "Starting torrents N of M" in the status bar. T1 measured at 472–498 ms (target 300). No skeleton state yet.
- [~] Shell: Windows title bar with menus, toolbar, sidebar (status, categories, tags, trackers with counts), virtual transfer list with all 38 columns (show and hide, sort), details tabs (General with piece map, Trackers, Peers, HTTP sources, Content, Speed graph), status bar that drops items as it narrows, torrent menu in qBittorrent's order with submenus. Not yet: column drag and resize, macOS frame.
- [~] Themes: Dose light and dark, neutral and tinted, six accents, three densities, UI scale (Ctrl+= / - / 0). Not yet: custom accent picker, app icon choice, motion.
- [~] Customization: status bar items (right-click the bar or View › Status bar). Not yet: toolbar order and label mode with drag and drop, details tab visibility.
- [~] Keyboard map (DESIGN.md, `secondary` = ⌘ on macOS), shortcuts sheet, settings sheet. Not yet: command palette (Ctrl+K focuses the filter), settings search.

### Phase 3: v1 feature parity
- [~] Adding: the Add torrent dialog for files, links, drops and magnets (metadata first), content layout, stop condition, add to top of queue, skip hash check, file selection, Never show again, and the "When adding a torrent" settings (`docs/associations.md`). Not yet: incomplete-torrent folder, delete .torrent files afterwards, merge trackers of duplicates, sequential and first-and-last pieces (librqbit).
- [ ] Every v1 item in the feature map, group by group: adding, list, details, speed, seeding, connection, disk, RSS, automation, app, theming, remote, tools, protocol, suggestions. The qBittorrent-compatible Web API is not started.

### Phase 3b: AI agents (MCP), the unique feature
- [~] `tt-mcp`: stdio (`ttorrent --mcp --scope read|manage|full`), off until Settings › AI agents is on, answers both MCP eras (2026-07-28 `server/discover` and 2025 `initialize`). Not yet: streamable HTTP, per-agent keys (the scope is a flag for now), Search scope.
- [~] Tools: `get_status`, `list_torrents`, `get_torrent`, `diagnose_torrent`, `seeding_report`, `plan_disk_cleanup`, `list_categories`, `list_tags`, `get_settings`, `get_skill` and the manage set. Not yet: `check_connection`, `get_stats`, RSS tools, `move_storage`, `set_file_priority`, `get_activity`.
- [ ] Events: completed, errored, stalled, disk low, ratio reached.
- [~] Safety: file deletion is never offered to agents and changes to more than 20 torrents are refused, until the in-app confirmation sheet exists. Not yet: the sheet, Activity log with Undo.
- [ ] Skill file and connect commands for Claude Code, Claude Desktop, Codex and Cursor in Settings (README has the Claude Code one).

### Phase 4: Windows release
- [ ] Signed installer, portable zip, Microsoft Store, winget, auto-update in place.
- [ ] Taskbar progress, notifications, jump list.

### Phase 5: macOS and Linux
- [ ] macOS: menu bar, Dock badge, notarized DMG, Homebrew cask.
- [ ] Linux: AppImage, deb, rpm, Flatpak, tray via StatusNotifierItem.

### librqbit change list

What tTorrent needs from librqbit 9.1, with the evidence. Fork or upstream; owner's call.

| Change | Why | Evidence |
|---|---|---|
| Per-chunk state only for pieces in flight | Tray-only budget (M2) | **Done** in t21dev/rqbit `sparse-chunk-status`, upstream PR [ikatson/rqbit#757](https://github.com/ikatson/rqbit/pull/757): −27 MB on 3.5 TB (log, 18 Nov) |
| Piece hashes of finished torrents on disk, read on demand; one copy of metadata for magnets | Tray-only budget (M2) | The remaining 20 B per piece; magnets keep the info dict twice (`torrent_file_from_info_bytes`) |
| Fast-resume that trusts file sizes instead of reading a sample of pieces (at least one per file) | Startup disk budget (D1) | **Done** in t21dev/rqbit `fastresume-file-sizes`, upstream PR [ikatson/rqbit#758](https://github.com/ikatson/rqbit/pull/758): 6.4 GB → 28 MB read at start (log, 18 Nov). Size only, not mtime: librqbit stores no mtimes |
| Lighter running torrents | M1, M2 | Every started torrent costs about 33 KB even with no peers; 735 started = most of the 88 MB tray-only |
| A public `SessionPersistenceStore` (SQLite instead of `session.json` rewritten on every change) | D4 writes, one database | `session_persistence` is a private module; 219.5 MB written in the first minute on the synthetic 735 (log, 18 Nov) |
| Restore torrents paused (today tTorrent rewrites `session.json` before start) | Waves without editing librqbit's file | `persist.rs` |
| Runtime per-torrent rate limits, force recheck, force reannounce | Torrent menu items that are greyed or answer "not available yet" | No public call in 9.1 |
| Protocol encryption, IP filter, BitTorrent v2, interface binding check | Feature map "verify" tags | To check in the spike |

## 4. Decisions

- **Licence: MIT** (18 November 2026), like Turbo Reader. Dependencies must be MIT/Apache-2.0 compatible; check new ones with `cargo deny`.
- **UI: GPUI** (18 November 2026), see §2.
- **MCP without rmcp** (18 November 2026): the stdio server is a small hand-written JSON-RPC loop in `tt-mcp`, run as `ttorrent --mcp`, a separate process that talks to the engine's local API. It answers both the 2026-07-28 revision (`server/discover`, per-request `_meta`) and 2025 clients (`initialize`). Nothing MCP-related loads into the engine. Revisit rmcp for the HTTP transport.
- **Two processes, one local API** (18 November 2026): `ttorrent` (engine, tray, `--mcp`) and `ttorrent-ui` (window) talk length-prefixed JSON over a named pipe (`tt-ipc`); the window, a second launch, the MCP server and `tt-bench` are all clients.
- **GPUI unpatched for now** (18 November 2026): crates.io gpui 0.2.2. Kitab's lean and start-up patches live in an AGPL repository; moving them into this MIT repository is the owner's call.

## 5. Open decisions

- Name: tTorrent is also the name of tagsoft's Android torrent client (since about 2012). Trademark check before branding.
- Landing site, following `kitab-landing`.
