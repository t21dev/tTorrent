# Verify before you say done

A checklist for agents and contributors. Work through it in order before reporting a change as finished. Paste the evidence (commands and their results) into your report. If a step doesn't apply, say so; if you couldn't run it, say that, and never report it as passed.

## 1. Did it run, not just compile?

- [ ] It builds without warnings (`cargo build`, `cargo clippy --all-targets -- -D warnings`), or for the mockup the script parses.
- [ ] **It runs.** Start the app (or open the mockup) and use the thing you changed. A syntax check is not a run: a page that parses can still be blank.
- [ ] No errors or panics in the log while you used it.

## 2. Tests

- [ ] `cargo test` passes for the touched crates.
- [ ] The rows of `TESTING.md` › "What each change must run" that apply to this change all pass.
- [ ] New behaviour has a test that fails without the change.

## 3. Look at it

- [ ] UI changes: the layout matrix passes (`TESTING.md`) and you **opened and looked at** the screenshots of the largest scale, the smallest window, light and dark, Windows and macOS frames.
- [ ] You checked by eye for what checkers miss: double focus rings, text wrapping where it shouldn't, clipped icons, inconsistent formats (dates, sizes), elements visible that should be hidden, misaligned columns.
- [ ] Keyboard: everything you added is reachable with Tab and arrows, has a visible focus ring, and its shortcut shows ⌘ on macOS and Ctrl elsewhere.
- [ ] Edge cases: an empty list, 5,000 torrents, a very long name, a non-Latin name, an error state, and the missing-files states if you touched files or storage.

## 4. Speed and memory

- [ ] For engine, storage, startup, list, rendering or window changes: `cargo xtask perf quick --against main`, with the comparison table pasted (`docs/perf/README.md`).
- [ ] No budget broken, and no regression without an owner-approved waiver.
- [ ] Perf experiments have a `docs/perf/log.md` entry, kept or rejected.

## 5. Rules that are easy to break (`AGENTS.md`)

- [ ] No default limit lowered to save memory or time.
- [ ] No colour, font, size or duration outside the theme tokens.
- [ ] No private data: nothing from `data/`, no real torrent names or paths from the owner's machine.
- [ ] No competitor name in user-facing copy except benchmarks and the qBittorrent importer.
- [ ] Destructive actions confirm or offer Undo; nothing deletes or re-downloads content on its own.

## 6. Docs in sync

- [ ] If behaviour changed, `DESIGN.md`, `PLAN.md` or the relevant `docs/*.md` say the same thing as the code.
- [ ] New settings, shortcuts, columns or menu items are in `DESIGN.md`.

## 7. Report honestly

Your report states, in short points:
- what changed;
- what you ran, with results (pass, fail, or not run and why);
- what you looked at by eye;
- what is left or uncertain.

"Should work" is not a result. If something failed, say so first.
