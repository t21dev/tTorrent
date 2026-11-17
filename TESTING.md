# Testing

What tests tTorrent has, what each one proves, and which ones a change must run. `VERIFY.md` is the checklist that uses them; `docs/perf/README.md` covers the speed and memory suite in depth.

## Layers

| Layer | Proves | Tool | Where it runs |
|---|---|---|---|
| **Unit** | A function does what it says, including edge cases | `cargo test` | Everywhere, every change |
| **Property** | Invariants hold for any input: piece bitfields, resume data round-trips, sort order, IPC and MCP message encoding | `proptest` | Everywhere, every change |
| **Integration (engine)** | The engine against real files and a real swarm: add, download, seed, pause, move, recheck, limits, queueing, import | `cargo test -p tt-engine --test '*'` with the local swarm (`tt-bench` fixtures) | Everywhere, every engine change |
| **Failure cases** | Missing files, unplugged drives, locked files, full disks, damaged database (`docs/missing-files.md`) | Integration tests on mounted VHDs (Windows), loop devices (Linux), disk images (macOS) | Reference machine; CI where the runner allows mounting |
| **Contract** | The Web API answers like the clients expect (Sonarr, Radarr, Prowlarr), and MCP tools match `docs/mcp.md` (schemas, scopes, refusals) | Recorded requests replayed against the engine; MCP client test harness | Everywhere |
| **UI end to end** | A real window: every control works by mouse and keyboard, shortcuts per OS, menus, sheets, customization, states | The app's automation hooks driven by a test runner, plus screenshots | Reference machine, per platform |
| **Layout** | Nothing overflows or is cut off: menus, submenus, sheets, status bar, table cells, at every scale, density, theme and platform | Layout checker over the running UI (see below) | Reference machine; mockups anywhere |
| **Accessibility** | Every control reachable by keyboard with a visible focus ring; contrast 4.5:1 text, 3:1 marks, in every theme, mode and accent | Automated contrast check over theme tokens; keyboard walk-through | Everywhere (tokens), reference machine (walk-through) |
| **Performance** | Budgets and no regressions (memory, CPU, startup, disk I/O, throughput, frame times) | `tt-bench` tiers 0 to 3 | Tier 0 in CI, tiers 1 to 3 on reference machines |
| **Soak** | Memory flat and nothing breaks over 24 hours and over 7 days | `cargo xtask perf nightly` | Reference machine, nightly and before release |

## The layout matrix

Every UI change is checked across this matrix (the mockup checker already does):

| Axis | Values |
|---|---|
| Platform frame | Windows, macOS |
| UI scale | 80, 100, 150 % |
| Density | Compact, Cozy, Comfortable |
| Mode | Light, Dark |
| Window | 1280 × 800 and the minimum window size |
| Content | Long names, every script (Latin, CJK, Devanagari, Tamil, Arabic), 5,000 torrents, all 38 columns |

What the checker asserts:
- every menu, submenu, sheet, popover and toast lies inside the window;
- no menu item, setting row, toolbar label, status bar item or table cell (except the ellipsized name, path, tracker and hash columns) is cut off or wraps;
- the status bar never scrolls or wraps; it drops totals, then labels, then optional items as it narrows;
- right-clicking at each corner of the list opens menus that stay fully visible, with submenus flipping at the edge;
- there are no runtime errors in the log.

It also writes screenshots of the worst cases (largest scale, smallest window) **that the agent must look at**. A passing checker plus an unread screenshot is not a pass: the status bar wrapping and a double focus ring were both caught only by looking.

## Mockup tests (now)

Until the app exists, the mockup is tested the same way:

```
# once, in any scratch folder
npm i jsdom@24 puppeteer-core@23

node scripts/mockup-smoke.js  docs/design-directions-2.html             # every feature clicked, no runtime errors
node scripts/mockup-layout.js <abs path to the html> <shots dir> [gd|hm|wk]   # the layout matrix, in real Chrome
```

Both must pass, and the screenshots must be reviewed, before the mockup is republished.

## Fixtures

- Generated, never real: synthetic libraries of 100, 735 and 5,000 torrents from a fixed seed, with sparse files; a local tracker and seeders on loopback; failure-case disks.
- **Never** use the owner's torrents, names, paths or statistics (`data/` stays private).
- Example content in tests and screenshots is open content: Linux ISOs, Blender open movies, Internet Archive items.

## Flaky tests

A test that fails without a code change is a bug, not noise. Fix it or quarantine it in the same day with an issue that says why; never retry until green. Timing assertions belong in `tt-bench`, not in unit or integration tests.

## What each change must run

| Change touches | Must run |
|---|---|
| Docs only | Links and file references resolve |
| Mockup | `mockup-smoke.js`, `mockup-layout.js` for all four directions, screenshots reviewed |
| Engine | Unit, property, integration, failure cases for the touched area, `tt-bench` tier 1 |
| Storage, startup, resume data | The above, plus D1 to D4 and the missing-files fixtures |
| UI | Unit, UI end to end for the touched area, the layout matrix, accessibility, `tt-bench` U1 to U3 |
| Web API or MCP | Contract tests, scope and refusal tests |
| Release | Everything, tier 3, both reference machines, all platforms |
