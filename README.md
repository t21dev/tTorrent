<div align="center">

# tTorrent

**A full-featured torrent client that stays fast, light and calm, however many torrents you keep.**

<sub>A desktop torrent client written in Rust on the librqbit engine, with a CPU-rendered window that is freed when you close it to the tray.</sub>

A **[t21 dev](https://github.com/t21dev)** project, by **[TriptoAfsin](https://github.com/TriptoAfsin)**, from the team behind [Turbo Reader](https://github.com/t21dev/turbo-reader) and [Kitab Reader](https://github.com/t21dev/kitab).

</div>

> **Status: planning.** There is no build yet. This repository holds the plan, the design and the mockups. Nothing below is a measured result.

## Why

Desktop torrent clients that do everything tend to get heavy: memory climbs with uptime, the disk thrashes at every start, and the interface hasn't moved on in years. tTorrent keeps the complete feature set and a layout you already know, and adds memory that stays flat, hard caps you can see and set, a calm modern interface, and a built-in way for AI agents to help.

Targets (to be measured, see [PLAN.md](PLAN.md)):

- Under 50 MB for everything, engine and window together, with hundreds of torrents and the window open.
- Never above 100 to 120 MB.
- The same on day 7 as on day 1.
- No disk spike at startup: no rechecks unless a file changed, torrents start in waves, the list shows in under 300 ms.
- No feature cuts: every limit keeps the usual default and stays a setting.

## Benchmarks

Numbers will come from `tt-bench` on the reference machines, against the most popular desktop client on the same library (method: [docs/perf/README.md](docs/perf/README.md), results: [docs/perf/log.md](docs/perf/log.md)). The starting point, measured on the owner's machine with 735 torrents: qBittorrent 5.2.4 starts near 200 MB and reaches about 1.3 GB with uptime.

## What it will have

- Everything a power user expects in v1: queueing, categories and tags, RSS rules, speed scheduling, seeding limits, proxy and interface binding, IP filtering, a torrent creator, a Web UI and more (see the feature map).
- Bring your library over: import torrents, progress, categories and tags from qBittorrent without rechecking.
- A familiar layout: toolbar, filter sidebar, a transfer list with 38 columns you can show, hide, reorder and sort, details tabs, status bar, tray menu and the shortcuts you already use.
- Light and dark, neutral or warm surfaces, six accents or your own, three densities, UI scale, and a choice of app icon.
- A toolbar you arrange yourself: drag to reorder, each button hidden, icon only, or icon and label.
- Native frames: the system menu bar on macOS, the title bar menu on Windows. ⌘ on macOS, Ctrl elsewhere.
- Start with Windows, default app for `.torrent` files and magnet links, keeps running in the tray.
- **A built-in MCP server for AI agents.** Ask Claude, Codex or Cursor why a torrent is stuck, what to remove to free 50 GB, or whether you're safe on your private trackers. Off by default, a key per agent, and file deletion always asks you first. See [docs/mcp.md](docs/mcp.md).
- A Web API that Sonarr, Radarr, Prowlarr and existing phone remotes already speak, so they work unchanged.

Windows first, then macOS and Linux.

## Docs

| File | What it is |
|---|---|
| [PLAN.md](PLAN.md) | Goals, memory budgets, architecture, phases, open decisions |
| [DESIGN.md](DESIGN.md) | The Dose design: tokens, layout, tray, keyboard map, customization |
| [TESTING.md](TESTING.md) | Test layers, the layout matrix, fixtures, and what each change must run |
| [VERIFY.md](VERIFY.md) | The checklist to finish before calling a change done |
| [docs/missing-files.md](docs/missing-files.md) | Deleted, moved or unplugged content: how each case is detected and fixed |
| [AGENTS.md](AGENTS.md) | Rules for anyone (or any agent) working in this repo |
| [docs/design-directions-2.html](docs/design-directions-2.html) | The interactive mockup (open in a browser) |
| [docs/feature-map.html](docs/feature-map.html) | Every planned feature, picked v1 / later / skip |
| [docs/perf/README.md](docs/perf/README.md) | Performance guideline: the benchmark suite, gates, and what to try when a change is slower |
| [docs/startup.md](docs/startup.md) | Starting with hundreds of torrents without a disk spike, and a list on screen in 300 ms |
| [docs/mcp.md](docs/mcp.md) | The MCP server for AI agents: use cases, scopes, tools, safety |

## Licence

To be decided.
