# Verifying a change

What "checked" means for each kind of change. A change is committed only after it builds cleanly and passes the checks for its kind. Commands run from the repo root in PowerShell unless they say otherwise.

## Every change

1. **Build** with no errors and no new warnings:
   ```
   cmake --build build --config Release --parallel
   ```
   (First time on a machine: `ttorrent\scripts\setup.ps1`, then `ttorrent\scripts\build.ps1`.) If the link fails because `ttorrent.exe` is in use, rename it to `ttorrent-old-<time>.exe` and build again. Windows allows renaming a running exe.
2. **Start it** in a scratch profile and check the log says `tTorrent <version> started`:
   ```
   build\Release\ttorrent.exe --profile=<scratch dir>
   ```
   A new profile goes straight to the main window. Never point `--profile` at the owner's real profile, and never start their qBittorrent.

## UI changes: screenshots

The app renders its own screenshots off-screen, so nothing on the owner's screen is captured:

```
$env:TTORRENT_UISHOT_DIR = "<empty folder>"
build\Release\ttorrent.exe --profile=<scratch dir> [--save-path=<folder> <file.torrent>]
```

It walks every main screen 1.5 s apart, saves a PNG of each and quits (`src/gui/dose/dosedev.cpp`):

- the main window and every menu;
- each details tab;
- Settings (first page, Advanced, Downloads and the automation group);
- About, the theme editor, the keyboard shortcuts window, the command palette, the memory card and Pair a phone;
- Statistics, Add link, Torrent creator, speed limits and cookies;
- the RSS and log tabs.

A `.torrent` on the command line also captures the Add dialog. Remove the variable afterwards.

Look at every screen the change touches, in a dark and a light theme (set `Appearance\Theme=light` in the scratch profile's `tTorrent.ini`) and, for spacing changes, in Dense and Standard density. Check for:

- clipped text;
- borders colliding;
- missing hover states;
- the system title bar showing anywhere;
- horizontal scrolling;
- colour literals that ignore the theme.

Use sample torrents with open-movie names (`ttorrent/scripts/test/make_sample_zip.py` makes one), never the owner's.

For a visible change on the product, show the owner a before/after image.

## Feature checks

Each feature built so far has a real-run check. Run the ones your change touches.

| Feature | Check |
|---|---|
| Webhooks | `python -I ttorrent\scripts\test\hook_listener.py <log> 8799`. Set `[tTorrent] WebhookUrl=http://127.0.0.1:8799/hook` in the scratch profile, add a sample torrent with `--skip-dialog=true`, and the log gets `added` and `finished` JSON. Use a port nothing else is listening on. |
| Unpack when finished | `python -I ttorrent\scripts\test\make_sample_zip.py <folder>`, set `UnpackArchives=true`, add the torrent with `--save-path=<folder>`. `Extras\` appears beside `Extras.zip`. |
| Low disk space guard | Set `LowDiskGiB` above the drive's free space and add a torrent with no data: the log says "Low disk space … stopped". Set it back to 5 and restart: "Space is back … started". |
| MCP server | Scratch profile with `McpEnabled=true`, `McpScope=manage` and `McpKeyHash=@ByteArray(<sha256 hex of a test key>)`, then `python -I ttorrent\scripts\test\mcp_check.py <key> --manage`. Every line must say ok. Use Python, not `curl` (a shell wrapper here cuts long output). |
| Health in the Add dialog | Add a public open-movie torrent (for example Sintel from webtorrent.io) with screenshots on; the Add dialog shows seeds and peers. |
| Update check | Run the screenshot pass twice: once as is (the private repo gives `update-message.png`, "the release list isn't public yet"), and once with `TTORRENT_UPDATE_REPO=qbittorrent/qBittorrent` (any public repo with newer tags gives the card in `update-window.png`). GitHub's API allows 60 anonymous requests an hour per address; past that the check falls back to `releases.atom`. |
| Rename, backup, restore, relocate | Self-test (no dialogs): `make_sample_zip.py <dir>\data`, then start with `TTORRENT_SELFTEST_DIR=<out>` and the sample torrent (`--skip-dialog=true --save-path=<dir>\data`). It renames a category and a tag, backs up, writes PASS/FAIL lines to `<out>\selftest.txt` and exits. Move `<dir>\data\Elephants Dream (2006)` to a renamed folder and start again with `TTORRENT_SELFTEST_MOVED=<that folder>`: relocate must find every file and reach 100 %. Write the backup's path into `config
estore-backup.pending`, start with `TTORRENT_SELFTEST_REPORT=1`: the save path, category and tag come back. |
| Check for updates | `TTORRENT_UPDATE_REPO=qbittorrent/qBittorrent` with the screenshot run: the card offers a newer version. Without it (private repo) the message says the release list isn't public. |
| Pair a phone, saved views, palette, shortcuts | The screenshot run shows each; click through them once by hand. |

## Benchmarks

Any claim that something is lighter or faster needs a benchmark, before and after.

**Setup.** `ttorrent\scripts\bench.ps1` runs qBittorrent and/or tTorrent, one at a time, on fresh copies of a profile copy (`-Source`, made once with the owner's permission; currently `T:\dev\ttorrent-bench\qb-profile-2026-11-18`, about 735 torrents). Each copy has:
- the Web UI off;
- downloads limited to 1 KiB/s;
- the window minimized to the tray;
- the app's log switched on.

**Nothing may point outside the scratch copy.** A copied profile keeps its absolute folder settings. Until 18 Nov 2026 its log folder pointed at the owner's real qBittorrent log folder, and the runs wrote there. `bench.ps1` now moves the log into the copy. Before using a new profile copy, check its `.ini` for other absolute paths: watched folders, export folders, the log.

**What it records.** It samples every second during start-up runs and every 15 s during long runs:
- Task Manager's memory (private working set);
- the working set and private bytes;
- CPU;
- disk read and written.

It keeps each app's log next to the results.

```
# Snapshot the build first, so later builds don't change what's measured:
#   copy build\Release\* to T:\dev\ttorrent-bench\tt-build-<HHmm>\
powershell -ExecutionPolicy Bypass -File ttorrent\scripts\bench.ps1 `
  -Source T:\dev\ttorrent-bench\qb-profile-2026-11-18 -Out T:\dev\ttorrent-bench\results-<date><letter> `
  -TtExe T:\dev\ttorrent-bench\tt-build-<HHmm>\ttorrent.exe -Apps both -StartupRuns 3 -SoakMinutes 30
```

- **Comparison with qBittorrent:** `-Apps both`, 30 minutes or more.
- **A/B of one change:** `-Apps tt`, 15 minutes each, same snapshot, one variable changed. The switches `TTORRENT_BENCH_NO_QSS` (no stylesheet) and `TTORRENT_BENCH_QSS_DROP=QMenu,QPushButton` (drops every rule naming those widgets) exist for that and do nothing otherwise.
- **Nothing else heavy while it runs.** Building in parallel can nudge CPU figures; say so in the report if it happened.

**Report.**
- One table: median, last-10-minutes average, peak, private bytes, CPU and disk.
- The machine, date, torrent count and run length.
- Which build each column was.

**Peaks come from start-up runs only** (`-StartupRuns 3`, sampled every second). Long runs sample every 15 s and can miss a short spike, so never compare their peaks (a 68 MB "saving" from 15 s samples was published and withdrawn on 2026-11-18). Single runs vary by a few MB, so call a difference of under about 5 MB "within noise" unless repeated. Start-up times come from the log: "Start-up: N torrents ready X s after launch" and "window shown X s after launch". The window line doesn't appear in benchmark runs, because the window stays in the tray.

## Releases

Only when the owner asks.

1. Bump `TTORRENT_VERSION` in `src/base/ttorrentversion.h` (and `TTORRENT_VERSION_LABEL` if it's no longer a preview).
2. Build, then run the screenshot pass and check the About dialog shows the new version.
3. Stage `build\Release` without `ttorrent-old*.exe`, add `COPYING` as `COPYING.txt`, and zip it as `tTorrent-<version>-windows-x64.zip`. Build the installer from the same folder with `ttorrent\scripts\make-installer.ps1 -Dist <staged folder> -Out tTorrent-<version>-windows-x64-setup.exe` (needs NSIS 3). Write `sha256.txt` with both files.
4. Unzip into an empty folder and start it with a scratch profile: it must start and listen. Then test the installer:
   - First back up the Start Menu `tTorrent.lnk` and `HKCU\Software\RegisteredApplications`; the test replaces and removes them.
   - Run `setup.exe /S /D=<scratch folder>` and start the installed exe with a scratch profile.
   - Uninstall with `uninstall.exe /S _?=<scratch folder>`: the folder, the `tTorrent` keys and the shortcuts must be gone.
   - Restore the backups.
5. Commit `chore(release): <version>`, tag `v<version>`, push both.
6. `gh release create v<version> -R t21dev/tTorrent --prerelease --title "tTorrent <version> (preview)" --notes-file <notes> <setup.exe> <zip> <sha256.txt>`.
   - The notes list what's new and what's left out.
   - Any figures come from a benchmark, with how they were measured and which build they came from.
7. Install locally with `ttorrent\scripts\install-local.ps1`.
