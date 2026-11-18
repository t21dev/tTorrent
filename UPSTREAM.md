# Taking in qBittorrent releases

tTorrent is qBittorrent with a new look and a few additions. `main` is built on a qBittorrent release tag, and our changes sit on top as ordinary commits. A new qBittorrent release comes in with a merge.

## One-time setup

```sh
git remote add upstream https://github.com/qbittorrent/qBittorrent.git
git fetch upstream --tags
```

## Taking a release

```sh
git fetch upstream --tags
git switch -c upstream/5.4.0 main
git merge release-5.4.0          # the qBittorrent tag
# resolve conflicts, build, run VERIFY steps in AGENTS.md
git switch main && git merge --ff-only upstream/5.4.0
```

Merge release tags (`release-X.Y.Z`), not qBittorrent's `master`: tags are tested and their Web API is stable.

## Keeping merges small

- **New code goes in new files.** Our UI lives in `src/gui/dose/` (theme, dialogs, toolbar) and `src/gui/titlebar/`. Upstream files get a call or a hook only: one line in `mainwindow.cpp`, `optionsdialog.cpp`, `addnewtorrentdialog.cpp`.
- **Mark every edit to an upstream file** with a `// tTorrent:` comment, so `git grep "tTorrent:"` lists them all when a merge conflicts.
- **Leave features out at build time, not by deleting code.** A feature we drop gets a CMake option (off by default) and `#ifdef`s around its entry points. Upstream's code for it stays, so their changes to it still merge cleanly.
- **The rebrand is a script** (`ttorrent/rebrand.py`). It is safe to run again after a merge and re-applies names that a merge brought back.
- **Strings stay translatable**, in qBittorrent's `tr()` contexts, so their translation updates keep working.

## Files we change upstream

`git grep -n "tTorrent:" -- src` lists the hooks. The rebrand touches `src/app/application.cpp`, `src/app/CMakeLists.txt`, `src/base/settingsstorage.cpp`, `src/qbittorrent.rc`, `src/icons/qbittorrent.ico` and a few message-box titles.
