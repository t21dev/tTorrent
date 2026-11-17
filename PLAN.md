# tTorrent plan

A torrent client with qBittorrent's features and layout, a modern finish, and memory that stays flat: engine and window together under 50 MB as the target, never above 100 to 120 MB. Windows first, then macOS and Linux.

Living references:
- Feature picks (v1 / Later / Skip): https://claude.ai/artifact/8rWBDxNJoz7gp5RDLCGwPo (105 v1, 18 later, 9 skipped = 93.2 % of qBittorrent 5.2, plus our additions)
- Design: `DESIGN.md`, mockup `docs/design-directions-2.html`

## 1. Goals and budgets

Every memory figure is **engine and window together**, with 735 torrents, measured as Task Manager memory (private working set) and as private bytes.

| Budget | Target | Hard limit | Measured how |
|---|---|---|---|
| Window open, 735 torrents | **under 50 MB** | 100 to 120 MB | Task Manager and private bytes, after 24 h, on an NVIDIA dGPU and an AMD/Intel iGPU |
| Window closed (tray only) | under 30 MB | 50 MB | Same |
| Over time | flat: day 7 equals day 1 within 10 % | no growth trend | `scripts/qb-mem-log.ps1`-style log |
| Idle CPU (seeding, window closed or open and idle) | 0 % | 1 to 2 % | Process Explorer, 10 min |
| Cold start to list on screen | under 300 ms | 500 ms | External timing, as in Kitab |
| Startup disk reads, 735 torrents, first 60 s (excluding pieces peers asked for) | under 50 MB | 200 MB | `scripts/qb-startup-io.ps1`; see `docs/startup.md` |
| Full hash checks at startup when no file changed | 0 | 0 | Engine log |
| Throughput | equal to qBittorrent on the same torrent | within 5 % | Same swarm, same machine |

**Baseline to beat** (this machine, 17 November 2026): qBittorrent 5.2.4 with 735 torrents uses ~214 MB private right after start and grows to ~1.3 GB with uptime. Its likely causes: libtorrent 2.x memory-mapped files, peer lists of up to ~3,000 per torrent, heap fragmentation, everything loaded at start. A 24-hour log is in `data/qb-mem.csv` (not committed).

**Rule:** no feature cuts for memory. Every limit (peer list size, connections, cache, checking) is a setting with qBittorrent's default. Savings come from how data is stored: compact peer records, pooled buffers with hard MB caps, piece hashes on disk for finished torrents, lazy loading, no memory-mapped I/O.

## 2. Architecture

### The memory floor of each UI stack

The window's rendering stack decides the budget. Measured on this machine where we have numbers:

| Stack | Engine + window, window open | Evidence | Look and motion | All scripts in names | Platforms | Verdict |
|---|---|---|---|---|---|---|
| Tauri + Rust | ~235 to 285 MB | **Measured** in Turbo Reader: 14 MB app process, ~220 to 270 MB with WebView2 | Easiest | Yes | Win, mac, Linux | Out |
| GPUI + Rust | ~55 to 80 MB Task Manager, ~110 to 175 MB private | **Measured** in Kitab (`../kitab/docs/perf/baseline.md`): GPUI alone in lean mode 87 MB private on NVIDIA, 149 MB on the AMD iGPU, plus ~70 MB GPU memory; engine estimated | Best | Yes | Win, mac, Linux | Meets 120 MB only on a dGPU; misses 50 MB |
| GPUI on WARP (software Direct3D) | ~60 to 75 MB private | **Measured** in Kitab: 39 MB GPUI floor, but ~14 cores busy for the first second and CPU on every frame | Best | Yes | Windows only | Fallback, not default |
| egui + wgpu or glow | Same GPU device and driver floor as GPUI | Not measured; same Direct3D/OpenGL drivers | Hard to make premium | Yes | Win, mac, Linux | Out |
| **iced + tiny-skia** (CPU rendering) | ~40 to 60 MB (estimate) | Not measured. Docs: lower memory than wgpu. No GPU device or driver loaded; a 1080p window needs ~16 MB of frame buffers; text by cosmic-text | Good; flexible styling | Yes (cosmic-text shapes complex scripts) | Win, mac, Linux | **Lead candidate** |
| Slint + software renderer | ~40 to 60 MB (estimate) | Not measured. Partial redraws. Docs: software renderer text is limited to western scripts | Good; animations built in | **No**: CJK, Devanagari and Tamil names would break | Win, mac, Linux | Out unless text support lands |
| Native per OS (Win32/AppKit/GTK) | ~25 to 40 MB (estimate) | SumatraPDF-class apps measure 15 to 40 MB | Hard to match the design three times | Yes | Three codebases | Out |

Estimates are marked; only measured rows are facts. The spike replaces every estimate.

### Proposed: one process, CPU-rendered window

```
 tTorrent (one process, Rust)
 ├─ engine: librqbit session, SQLite, limits, RSS, scheduler, Web API
 ├─ tray / menu-bar icon (native, no GPU)
 └─ window: iced + tiny-skia, created when shown, destroyed when hidden
            (frame buffers, glyph cache and widget tree freed on close to tray)
```

Why:
- A GPU stack costs 87 to 149 MB before we draw anything, which alone breaks a 120 MB budget on integrated GPUs. CPU rendering has no device or driver cost; a torrent list redraws about once a second and only where something changed, so the CPU cost is small.
- One process instead of an engine plus a UI process saves a second runtime and the IPC layer. Destroying the window on close gives the same "tray costs almost nothing" benefit.
- The engine stays a separate crate behind a typed API, so the Web API, a CLI or a different window can drive it.

Risks to check in the spike: smoothness of 220 ms transitions on a 4K window on the CPU (fall back to shorter or fewer effects), shadow and blur cost (DESIGN.md already avoids blur), iced's animation support, and font fallback for mixed-script names.

### Spike decides between

1. **iced + tiny-skia** (lead), and
2. **GPUI lean** (comparison, since Kitab already proves its polish),

on the same screen: 735-row list, sidebar, details with the piece map, status bar, one open menu. Measured on the NVIDIA dGPU and the AMD iGPU, window open and closed. The stack that meets **100 MB hard / 50 MB target** wins; if both do, the lighter one wins.

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
  tt-ui         the window (iced + tiny-skia, or GPUI if the spike says so)
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
- [ ] The same window built twice, iced + tiny-skia and GPUI lean, with a 735-row virtual list, sidebar, details and piece map: memory and CPU on the NVIDIA dGPU and the AMD iGPU, window open and closed.
- [ ] Decide the UI stack against the budget (100 MB hard, 50 MB target, engine and window together); list of librqbit changes.

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
