# Agent instructions

tTorrent is a torrent client (C++, Qt 6, libtorrent), GPL. It began as a fork of qBittorrent and is being rebuilt from there to be leaner, faster and cleaner. Product copy (README, site, app) never pitches it against qBittorrent; qBittorrent appears only in credits (About, README credits, installer) as where tTorrent started. The Rust/GPUI prototype is on the `rust-prototype` branch (checked out at `../tTorrent-prototype`), for reference only.

Three files set how we work. Read the one you need before you start:

| File | Covers |
|---|---|
| this file | Rules, the change loop, what the owner decides |
| [DESIGN.md](DESIGN.md) | How tTorrent looks: tokens, type, density, components, icons, motion |
| [VERIFY.md](VERIFY.md) | How a change is checked: build, screenshots, feature tests, benchmarks, releases |

Also: [ROADMAP.md](ROADMAP.md) (what's planned and done), [../UPSTREAM.md](../UPSTREAM.md) (staying mergeable).

## Rules that are easy to break

- **Stay mergeable.** New code goes in new files (`src/gui/dose/`, `src/gui/titlebar/`, `src/base/` for non-GUI code). Upstream files get one-line hooks marked `// tTorrent:`. Features we drop are switched off in `src/gui/dose/dosetrim.cpp`, never deleted (`../UPSTREAM.md` says when a build-time switch is worth it).
- **The look comes from tokens.** Colours come from `src/gui/dose/theme/themes/*.json` through `DoseThemeSource`; sizes from its density table. `stylesheet.qss` is a template with `@token@` placeholders and no colour literals. A colour that must not follow the theme (the QR code's white) is set in code with a comment saying why. Themes and accent maths follow Kitab (`../../kitab/DESIGN.md`, a sibling checkout), so the two apps match.
- **Measure, never guess.** Every memory, CPU or start-up figure in docs, release notes, the README or the site comes from a run in [VERIFY.md](VERIFY.md) › Benchmarks, with the machine, date and method beside it. Report Task Manager's memory (private working set) and private bytes together. Figures worked out from code are labelled "estimate". If you couldn't measure it, say so.
- **Never touch the owner's own qBittorrent** (its process, profile or settings). Test with `ttorrent.exe --profile=<scratch dir>`. Benchmarks use a copy of the profile, made with the owner's permission.
- **Never capture the owner's screen.** Screenshots come from the app's own off-screen renderer (`TTORRENT_UISHOT_DIR`, see [VERIFY.md](VERIFY.md)).
- **Private data stays out of git.** Use open content (Linux ISOs, Blender open movies) in docs, tests and screenshots, never torrent names, IP addresses or paths from the owner's machine.
- **Credit qBittorrent** in the About dialog, README and installer. Keep upstream copyright headers; new files carry a `t21 dev` GPL header.
- **Private trackers must keep working.** libtorrent's peer ID and tracker user agent stay qBittorrent's (`QBT_VERSION`). tTorrent's own version (`src/base/ttorrentversion.h`) is only shown to people.

## The change loop

1. Read the code you'll change, and the relevant part of DESIGN.md.
2. Make the change: a new file plus a `// tTorrent:` hook where you can.
3. Build: no errors and **no new warnings**.
4. Check it the way [VERIFY.md](VERIFY.md) says for that kind of change: screenshots for UI, a real run for features, a benchmark for anything said to save memory or time.
5. Commit only after a clean build and a check, with a conventional message (`feat(ui): …`, `fix(theme): …`, `perf(ui): …`, `chore(release): 0.2.0`). No attribution lines.
6. Push to `main` and install locally (`ttorrent\scripts\install-local.ps1`): the owner asked for this after every verified change, so they can try it. Tags and GitHub releases only when the owner asks.

A change isn't done until the app starts in a scratch profile with it.

## The owner decides

- **Which features stay, go or change their defaults.** Offer the choice as an Artifact (the cuts list, the resource-trims list) with what each one costs, then act on the picks. Never cut a feature or lower a limit for lightness on your own.
- **Looks.** Show options (mock-ups or screenshots) before a big visual change. Known preferences: flat buttons with depth from edges and fills, never gradients; icon buttons and icon labels wherever they fit; compact and crisp, never gimmicky; a hover state on everything clickable.
- **Anything outward.** Releases, tags, where updates and bug reports go, publishing anything.

## Code

- Follow qBittorrent's `CODING_GUIDELINES.md`: `u"..."_s` literals (they need `base/global.h`), braces on their own line, `tr()` for user-visible text.
- Never call `QMainWindow::menuBar()`: it creates a new menu bar and replaces the custom title bar. Find the existing one with `findChild<QMenuBar *>()`.
- Our own settings live under `tTorrent/…` or `Appearance/…` in `SettingsStorage`.
- Helper scripts that will run again go in `ttorrent/scripts/` (tests in `ttorrent/scripts/test/`), not in a temp folder.
