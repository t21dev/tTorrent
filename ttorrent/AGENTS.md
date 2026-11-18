# Agent instructions

tTorrent is a fork of qBittorrent (C++, Qt 6, libtorrent) with a new look and a few additions. GPL, like upstream. The Rust/GPUI prototype that came before is on the `rust-prototype` branch, for reference only.

## Rules that are easy to break

- **Stay mergeable.** Read `../UPSTREAM.md`. New code goes in new files (`src/gui/dose/`, `src/gui/titlebar/`); upstream files get one-line hooks marked `// tTorrent:`. Features we drop are switched off at build time (CMake option plus `#ifdef`), never deleted.
- **The look comes from tokens.** Colours come from `src/gui/dose/theme/themes/*.json` through `DoseThemeSource`; sizes from its density table. `stylesheet.qss` is a template with `@token@` placeholders, so it has no colour literals. Themes and accent maths follow Kitab (`../../kitab/DESIGN.md` (a sibling checkout)), so the two apps match.
- **Measure, never guess.** Every memory, CPU or startup figure in docs, the README or the site comes from a run you can repeat, with the machine, date and method next to it. Compare Task Manager's private working set, the working set and private bytes. If you couldn't measure it, say so.
- **Never touch the owner's own qBittorrent** (its process, profile or settings). Test with `ttorrent.exe --profile=<scratch dir>`.
- **Private data stays out of git.** Use open content (Linux ISOs, Blender open movies) in docs, tests and screenshots, never torrent names from the owner's machine.
- **Credit qBittorrent** in the About dialog, README and installer. Keep upstream copyright headers; new files carry a `t21 dev` GPL header.

## Code

- Follow qBittorrent's `CODING_GUIDELINES.md`: `u"..."_s` literals, braces on their own line, `tr()` for user-visible text.
- Build: `ttorrent\scripts\build.ps1`. It must build with no new warnings, and the app must start in a scratch profile before a change counts as done.
- Conventional commits: `feat(ui): …`, `fix(theme): …`, `chore(upstream): merge release-5.4.0`.
- Don't commit or push unless the owner asks.
