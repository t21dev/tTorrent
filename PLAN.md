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

## 3. Phases

### Phase 0: spike (1 to 2 weeks)
- [ ] librqbit headless: load a **copy** of the 735-torrent `BT_backup`, seed for 24 h, log memory every 5 min alongside qBittorrent's log. No downloading of new content needed.
- [ ] Measure: private bytes, Task Manager memory, idle CPU, startup time, handles, threads.
- [ ] Baseline qBittorrent's startup I/O with `scripts/qb-startup-io.ps1`, then the same for the librqbit spike.
- [ ] Check each "verify" tag in the feature map against librqbit's source (encryption, IP filter, v2, web seeds, PEX, LSD, move storage, per-torrent limits).
- [ ] A GPUI window (lean mode) over IPC with a 735-row virtual list, sidebar, details and piece map: Task Manager memory, private bytes, GPU memory and CPU on the NVIDIA dGPU and the AMD iGPU, window open, and the engine alone with the window closed.
- [ ] Check the window-open and tray-only budgets on both GPUs; list the librqbit changes.

### Phase 1: engine
- [ ] Session, SQLite state, resume data, queueing, speed and seeding limits, slow mode and scheduler.
- [ ] Hard memory caps (buffers, cache, peers) as settings; memory shown in the status bar.
- [ ] Startup without a disk spike (`docs/startup.md`): trusted resume data (size and mtime, no rechecks), waves in priority order, one sorted I/O queue per physical disk with HDD/SSD detection, files opened on demand, background I/O priority, incremental resume writes.
- [ ] qBittorrent import.
- [ ] Missing and moved content (`docs/missing-files.md`): drive unavailable vs deleted, Locate…, download missing files, can't-access retries, disk full, database backups.
- [ ] Single instance, file and magnet association, autostart, tray with the qBittorrent-identical menu (DESIGN.md).

### Phase 2: window
- [ ] List from the SQLite snapshot in under 300 ms, rows going live in waves, startup progress in the status bar, skeleton only with no snapshot (`docs/startup.md`).
- [ ] Shell: menus per OS, toolbar, sidebar, transfer list (virtual, sortable, all qBittorrent columns), details tabs, status bar.
- [ ] Themes: Dose light and dark, neutral and tinted, six accents and custom, density, scale, app icon choice.
- [ ] Customization: toolbar order and label mode with drag and drop, status bar and tab visibility.
- [ ] Keyboard map per OS, shortcuts sheet, command palette, settings with search.

### Phase 3: v1 feature parity
- [ ] Every v1 item in the feature map, group by group: adding, list, details, speed, seeding, connection, disk, RSS, automation, app, theming, remote, tools, protocol, suggestions.

### Phase 3b: AI agents (MCP), the unique feature
- [ ] `tt-mcp` with rmcp: stdio (`ttorrent --mcp`) and streamable HTTP on localhost, per-agent keys with Read, Manage, Full and Search scopes.
- [ ] Agent-shaped tools: `diagnose_torrent`, `plan_disk_cleanup`, `seeding_report`, `check_connection`, plus the full manage set (docs/mcp.md).
- [ ] Events: completed, errored, stalled, disk low, ratio reached.
- [ ] Settings › AI agents, in-app confirmation for file deletion and bulk changes, Activity log with Undo.
- [ ] Skill file and connect commands for Claude Code, Claude Desktop, Codex and Cursor.

### Phase 4: Windows release
- [ ] Signed installer, portable zip, Microsoft Store, winget, auto-update in place.
- [ ] Taskbar progress, notifications, jump list.

### Phase 5: macOS and Linux
- [ ] macOS: menu bar, Dock badge, notarized DMG, Homebrew cask.
- [ ] Linux: AppImage, deb, rpm, Flatpak, tray via StatusNotifierItem.

## 4. Open decisions

- Licence (MIT like Turbo Reader, or AGPL like Kitab; librqbit is Apache-2.0, so both work).
- Name: tTorrent is also the name of tagsoft's Android torrent client (since about 2012). Trademark check before branding.
- Landing site, following `kitab-landing`.
