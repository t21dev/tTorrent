# tTorrent for AI agents (MCP)

tTorrent has a built-in [Model Context Protocol](https://modelcontextprotocol.io) server, so Claude Code, Claude Desktop, Codex, Cursor and any other MCP client can see what your torrents are doing, explain problems in plain words, and, with a key that allows it, manage them for you.

It is off until you turn it on. Everything stays on your computer: tTorrent sends nothing anywhere, and the agent you connect decides what it does with what it reads.

Status: partly built (18 November 2026). `ttorrent --mcp --scope read|manage|full` serves stdio today, for both the 2026-07-28 revision and 2025 clients; it is a small process that talks to the engine's local API. Working: the switch in Settings › AI agents (off by default), the read tools above except `check_connection`, `get_stats`, `get_activity` and the RSS tools, and the manage set except `move_storage` and `set_file_priority`. Not yet: the HTTP transport, per-agent keys (the scope is a command-line flag until then), events, Activity with Undo, and the in-app confirmation sheet. Until that sheet exists, agents can never delete files and changes to more than 20 torrents are refused. Plan: `PLAN.md` › Phase 3b.

Connect Claude Code (read-only):

```
claude mcp add ttorrent -- "<folder>\ttorrent.exe" --mcp --scope read
```

## Why this is ours

MCP servers for qBittorrent exist, but all of them are separate add-ons: you enable qBittorrent's Web UI, give the add-on your Web UI password, and it scrapes the same API a browser uses. tTorrent ships it inside the engine:

| | qBittorrent + an add-on MCP server | tTorrent |
|---|---|---|
| Setup | Enable the Web UI, install Python or a binary, store the Web UI password in a config file | Settings › AI agents › turn on, copy one command |
| Access control | One Web UI login with full control | A key per agent, each Read, Manage or Full |
| What the agent sees | Raw Web API fields | Agent-shaped tools: plain-language diagnosis, disk-space plans, ratio reports, live events |
| Destructive actions | Whatever the Web UI allows | Deleting files asks you in the app first, unless you allowed it for that key |
| Record of what agents did | None | Activity log in the app, each action with Undo where possible |
| Notifications | Polling | Events: completed, errored, stalled, disk low |
| Cost when off | n/a | Nothing loaded; when on, a few MB inside the memory budget |

## Use cases

Ordered by how often people would use them.

| # | You say | What the agent does | Tools |
|---|---|---|---|
| 1 | "What's downloading and when will it finish?" | Summarises active torrents, speeds and ETAs | `get_status`, `list_torrents` |
| 2 | "Why is this one stuck at 88 %?" | Reads seeds, availability, tracker errors, port and disk state, and explains the cause and the fix | `diagnose_torrent` |
| 3 | "Free up 50 GB." | Proposes finished torrents to remove (largest, oldest, ratio met, not needed on private trackers) and waits for your yes | `plan_disk_cleanup`, `remove_torrents` |
| 4 | "Am I safe on my private trackers?" | Per tracker: ratio, seeding time and torrents that still need seeding before you remove them | `seeding_report` |
| 5 | "Put all Linux ISOs in a category and move them to D:\ISOs." | Matches by name, sets category, moves files while running | `list_torrents`, `set_category`, `move_storage` |
| 6 | "Download this magnet into Films and tell me when it's done." | Adds it with the right category and save path, then reports on the completion event | `add_torrent`, events |
| 7 | "Slow down while I'm on a call" / "Pause everything until 1 am." | Turns on slow mode, or pauses with a timed resume | `set_speed_mode`, `pause_all` |
| 8 | "Make a rule that grabs new 1080p episodes of this feed." | Writes the RSS rule, previews which articles it would match, then saves it | `preview_rss_rule`, `save_rss_rule` |
| 9 | "Weekly summary." | Uploaded, downloaded, ratio, top seeders, problems | `get_stats` |
| 10 | "Is my VPN binding and port forwarding working?" | Checks interface binding, port state, encryption and connectability | `check_connection` |
| 11 | "Stop seeding Films at ratio 2." | Changes the setting, says exactly what changed | `get_settings`, `update_settings` |
| 12 | "When a book finishes, add it to Kitab." | Chains tTorrent's completion event with another tool the agent has (t21 apps work together) | events, `get_torrent` |

Not offered: searching torrent sites. tTorrent has no built-in site search, and MCP doesn't add one. If you connect your own Prowlarr or Jackett in Settings, the agent can use that search only with a key that has the separate Search permission.

## Settings › AI agents

- **Allow AI agents to connect** (off by default).
- **Address**: `http://127.0.0.1:7812/mcp`, localhost only. The stdio transport (`ttorrent --mcp`) needs no network at all.
- **API keys**: name, scope (Read, Manage, Full), optional Search permission, created and last used. Shown once on creation, stored hashed.
- **Ask before deleting files** (on): a removal that deletes files shows an in-app prompt naming the agent and the torrents. A key can be allowed to skip it.
- **Connect an agent**: ready-to-copy commands for Claude Code, Claude Desktop, Codex and Cursor with the address and key filled in.
- **Activity**: every write an agent made, newest first, with Undo for pause, category, tags, limits and moves.

## Scopes

| Scope | Allows |
|---|---|
| Read | Every read tool and events. Nothing changes. |
| Manage | Read, plus add, pause, resume, force start, recheck, reannounce, categories, tags, limits, slow mode, move, rename, file priorities, RSS rules, settings. Remove only without deleting files. |
| Full | Manage, plus removing torrents with their files (still asks in the app unless allowed). |
| Search (add-on) | Search through your own Prowlarr or Jackett, if configured. |

## Tools (v1)

Names are snake_case, results are JSON with plain-language `summary` fields an agent can quote. Lists page with `limit` and `cursor`. Sizes are bytes plus a formatted string; times are ISO 8601.

**Read**

| Tool | Returns |
|---|---|
| `get_status` | Speeds, active, seeding, errored and total counts, free disk space, port state, slow mode, memory used against its cap |
| `list_torrents` | Torrents with filter (status, category, tag, tracker, text), sort (any of the 38 columns) and chosen fields |
| `get_torrent` | Everything in the details tabs: general, trackers, peers summary, web seeds, files |
| `diagnose_torrent` | Causes in order of likelihood (no seeds, tracker error, port closed, disk full, file locked, stalled peers), each with a fix |
| `seeding_report` | Per tracker and per torrent: ratio, seeding time, what still needs seeding |
| `plan_disk_cleanup` | Given a target in GB, a ranked list of finished torrents to remove and why; changes nothing |
| `get_stats` | Totals for a period: downloaded, uploaded, ratio, top torrents |
| `list_categories` / `list_tags` | With counts and save paths |
| `list_rss` / `preview_rss_rule` | Feeds, rules, and which articles a draft rule would match |
| `check_connection` | Port reachability, interface binding, proxy, encryption, DHT nodes |
| `get_settings` | The settings an agent may read (never keys or passwords) |
| `get_activity` | What agents changed and when |
| `get_skill` | A short guide for agents on using tTorrent well |

**Manage**

`add_torrent` (magnet, URL or file path; category, tags, save path, paused, sequential), `pause` / `resume` / `force_start` (ids or a filter), `pause_all` (optional `until`), `set_category`, `set_tags`, `set_limits` (torrent or global), `set_speed_mode` (normal, slow, with optional schedule), `move_storage`, `rename`, `set_file_priority`, `recheck`, `reannounce`, `queue_move`, `save_rss_rule`, `update_settings`, `remove_torrents` (without files).

**Full**

`remove_torrents` with `delete_files: true`.

**Events** (resource subscriptions over the HTTP transport)

`torrent_completed`, `torrent_errored`, `torrent_stalled`, `disk_low`, `ratio_reached`.

## Safety rules

- Off by default; localhost only; no key, no access.
- Agents never see API keys, Web UI passwords, proxy passwords or tracker passkeys (announce URLs are shown with the passkey masked).
- Writes are rate limited per key and recorded in Activity.
- Bulk changes over 20 torrents and every file deletion are shown in the app before they run, unless the key allows it.
- Tool descriptions say what will change, so the agent can ask you first.

## Budget

The MCP server lives in the engine (`rmcp`, as in Turbo Reader). When it is off, nothing is loaded. When on, it must stay inside the overall memory budget in `PLAN.md`; measure it with the rest.
