# Agent instructions

tTorrent is a torrent client with qBittorrent's features and layout, a modern finish, and memory that stays flat. Rust and librqbit, with a GPUI window (decided). Read `PLAN.md` for the architecture and phases and `DESIGN.md` before touching any UI. **Before reporting any change as done, work through `VERIFY.md`**; `TESTING.md` says which tests each kind of change must run, and `docs/perf/README.md` how speed and memory are compared.

## Rules that are easy to break

- **No feature cuts for memory.** Every limit (peer list size, connections, cache, checking, columns) keeps qBittorrent's default and is a user setting. Memory savings must come from how data is stored: compact records, pooled buffers with hard MB caps, data on disk instead of in RAM, lazy loading. A change that lowers a default to save memory is a regression.
- **Measure, never guess.** Every memory, CPU or start-up figure in docs, the README, the app or the site comes from a run you can repeat, with the machine, date and method written next to it (see `../kitab/docs/perf/baseline.md` for the format). Compare Task Manager (private working set), working set and private bytes. If you couldn't measure it, say so.
- **Familiar first.** Layout, menu items, column names, tray menu and shortcuts follow qBittorrent unless DESIGN.md says otherwise. Shortcuts use ⌘ on macOS and Ctrl elsewhere, everywhere.
- **The UI follows DESIGN.md.** Colours, fonts, sizes and durations come from `tt-theme` tokens, never literals. If the code and DESIGN.md disagree, fix one of them on purpose in the same change.
- **The engine never loads the GPU.** Torrents, tray, Web API, MCP, RSS and scheduling live in `tt-engine`; the GPUI window is the separate `tt-ui` process, started when shown and ended when closed to the tray. Budgets (Task Manager, both processes together, 735 torrents): window open 100 MB target, 120 MB hard; tray only under 30 MB, 50 MB hard.
- **Private data stays out of git.** `data/` holds the owner's qBittorrent measurements and torrent statistics. Never commit it, and never put torrent names from the owner's machine in docs, tests, fixtures or screenshots. Use open content (Linux ISOs, Blender open movies, Internet Archive) for examples.

## Code

- MIT licence. Only add dependencies with MIT, Apache-2.0, BSD, ISC or Zlib licences; `cargo deny check licenses` must pass.
- Rust 2024, `unsafe_code = "deny"` and clippy `unwrap_used` / `expect_used` as warnings, like Kitab. Release profile: thin LTO, one codegen unit, stripped symbols.
- `cargo fmt`, `cargo clippy --all-targets`, and `cargo test` pass before a change is done.
- Conventional commits, as in Kitab: `feat(engine): …`, `fix(ui): …`, `docs(perf): …`.
- Don't commit or push unless the owner asks.

## Build artifact cleanup (disk space)

Rust build output grows into tens of GB and Cargo never prunes it. Clean up **only when** the task is finished, its builds and tests have passed, and no installer under `target/release/bundle` is waiting to be collected (ask first if one is). Never clean up partway through a task.

Stop any running app, `cargo` or `tt-*` processes first (Windows locks files in use), then run `cargo clean` at the repo root. Agent worktrees under `.claude/worktrees/`: once merged, check `git -C <worktree> status`. Remove them with `git worktree remove <path>` only if they have no uncommitted changes.

Afterwards, tell the owner what was removed and roughly how much space was freed.
