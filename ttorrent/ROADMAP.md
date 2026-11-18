# tTorrent roadmap

The owner's picks from 18 November 2026 (feature list artifact, `choices/main`).

## Left out (switched off in `dose/dosetrim.cpp`, code kept for clean merges)

Update checker (replaced by tTorrent's own updates), Donate menu item, search engine, peer host names, embedded tracker, I2P, email on completion (replaced by webhooks), alternative Web UI.

Kept as they are: Web UI and Web API, RSS, run a program on add or finish, watched folders, IP filter lists, peer countries, dynamic DNS, lock, execution log, statistics, speed graphs, torrent creator, preview, cookie manager, all translations.

## First release

| Feature | What it is |
|---|---|
| Memory that stays flat | Hard caps on caches and queues (settings, not cuts), memory returned when idle, an uptime memory graph |
| Fast start | The torrent list shows at once; trackers, peers and file details load in the background without a disk spike |
| MCP server | AI agents (Claude Code, Codex, Cursor) read and, with a key that allows it, manage torrents. Off by default, local only. Design: the prototype's `docs/mcp.md`. **Done, v1** (Settings › Web UI › AI agents; HTTP on 127.0.0.1:7812/mcp, one key, read or manage, 13 tools; 2026-07-28 and 2025 clients) |
| Import from qBittorrent | Torrents, progress, categories, tags and settings in one step, no recheck |
| tTorrent updates | Signed updates of our own, installed on restart |
| Low disk space guard | Pauses downloads before the drive fills, resumes when space returns. **Done** (Settings › Downloads) |
| Command palette | Ctrl+K: torrents, actions, settings. **Done** (actions; Ctrl+/ lists and changes every shortcut) |
| Notifications with actions | Finished or failed, with Open folder and Show. **Done** (a card in the window; the system notification is unchanged) |
| Saved views | A filter and sort saved as a sidebar entry. **Done** (sidebar › Views) |
| Health in the Add dialog | Seeds and peers before adding. **Done** (tracker scrape, HTTP and UDP) |
| Webhooks | A URL called on added, finished, failed. **Done** (JSON POST; Settings › Downloads) |
| Unpack when finished | .zip, .rar, .7z extracted into the save folder. **Done** (Windows' own tar.exe) |
| Pair a phone | QR code that opens the Web UI on the same network. **Done** (Tools › Pair a phone) |

## Later

Auto-sort rules, portable mode. (Custom shortcuts: done.)

## Not planned

Activity history with Undo, duplicate check, visual speed schedule, VPN kill switch.
