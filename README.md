<div align="center">

# tTorrent

**qBittorrent's features and layout, with a modern finish and memory that stays flat.**

<sub>A desktop torrent client written in Rust on the librqbit engine, with a CPU-rendered window that is freed when you close it to the tray.</sub>

A **[t21 dev](https://github.com/t21dev)** project, by **[TriptoAfsin](https://github.com/TriptoAfsin)**, from the team behind [Turbo Reader](https://github.com/t21dev/turbo-reader) and [Kitab Reader](https://github.com/t21dev/kitab).

</div>

> **Status: planning.** There is no build yet. This repository holds the plan, the design and the mockups. Nothing below is a measured result.

## Why

qBittorrent does everything, but on a busy machine its memory climbs with uptime: on the owner's machine, 735 torrents start near 200 MB and reach about 1.3 GB after a while, then drop back after a restart. tTorrent aims to keep the whole feature set and the familiar layout while keeping memory flat, with hard caps you can see and set.

Targets (to be measured, see [PLAN.md](PLAN.md)):

- Under 50 MB for everything, engine and window together, with hundreds of torrents and the window open.
- Never above 100 to 120 MB.
- The same on day 7 as on day 1.
- No feature cuts: every limit keeps qBittorrent's default.

## What it will have

- About 93 % of qBittorrent 5.2's features in v1, plus import of your qBittorrent torrents, categories and tags without rechecking.
- qBittorrent's layout: toolbar, filter sidebar, transfer list with all 38 columns, details tabs, status bar, the same tray menu and shortcuts.
- Light and dark, neutral or warm surfaces, six accents or your own, three densities, UI scale, and a choice of app icon.
- A toolbar you arrange yourself: drag to reorder, each button hidden, icon only, or icon and label.
- Native frames: the system menu bar on macOS, the title bar menu on Windows. ⌘ on macOS, Ctrl elsewhere.
- Start with Windows, default app for `.torrent` files and magnet links, keeps running in the tray.
- A qBittorrent-compatible Web API, so Sonarr, Radarr, Prowlarr and phone remotes work unchanged.

Windows first, then macOS and Linux.

## Docs

| File | What it is |
|---|---|
| [PLAN.md](PLAN.md) | Goals, memory budgets, architecture, phases, open decisions |
| [DESIGN.md](DESIGN.md) | The Dose design: tokens, layout, tray, keyboard map, customization |
| [AGENTS.md](AGENTS.md) | Rules for anyone (or any agent) working in this repo |
| [docs/design-directions-2.html](docs/design-directions-2.html) | The interactive mockup (open in a browser) |
| [docs/feature-map.html](docs/feature-map.html) | Every qBittorrent feature, picked v1 / later / skip |

## Licence

To be decided.
