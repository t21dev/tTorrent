# Starting with a long torrent list

What happens when tTorrent starts with hundreds of torrents, and how it avoids the disk spike and the slow, jumpy first screen.

Status: specification for Phase 1 (engine) and Phase 2 (window). Every target here is measured with `scripts/qb-startup-io.ps1` (qBittorrent baseline) and the same method on tTorrent.

## Why qBittorrent spikes the disk at startup

With 735 torrents (1.57 TB in 2,063 files), a qBittorrent start does several heavy things at once:

| # | What happens | Disk cost |
|---|---|---|
| 1 | Loads every `.fastresume` and `.torrent` file | Small (~23 MB), but 1,470 file opens |
| 2 | Checks every torrent's files against its resume data (exists, size, timestamps) | Thousands of metadata reads |
| 3 | Any torrent whose resume data doesn't match goes to a **full hash check**, reading the whole file. With "max active checking torrents" at 10 (your setting; the default is 1), ten run at once | Up to the full size of the data, in parallel, random on a hard disk |
| 4 | **All 735 torrents start at once**: every one announces, peers connect, and requests for pieces arrive for hundreds of different files | A burst of random reads across the whole library |
| 5 | libtorrent 2.x reads through memory-mapped files, so those reads also fill RAM | Disk and memory spike together |
| 6 | Antivirus scans each file the moment it is opened | Multiplies 4 |

Points 3 and 4 are the spike. Neither is needed to get back to where you were.

## What tTorrent does instead

None of this lowers a limit or removes a feature. The end state (every torrent running, same connections, same speeds) is reached within about a minute; only the order and pacing change.

### 1. Trust verified resume data
- The engine stores, per torrent, the verified piece bitfield plus each file's size and modification time in SQLite (one database, not 1,470 files).
- At startup it only reads file metadata (one directory listing per folder, not one open per file). A full hash check runs only if a file's size or modification time changed, or the file is missing.
- **Importing from qBittorrent never rechecks**: its `.fastresume` piece data is trusted the same way.

### 2. Start in waves, most important first
- Wave order: downloading, then checking and queued, then seeding torrents with recent upload activity, then idle seeds, then stopped (which stay stopped).
- A wave of about 25 torrents starts every 1 to 2 seconds; the next wave waits while the disk queue is busy.
- Setting: Settings › Behaviour › **Start torrents gradually** (on). Off starts everything at once, as qBittorrent does.

### 3. One disk queue per physical disk
- All reads and writes for a drive go through one queue that sorts requests by file and offset and merges neighbours, so a hard disk sweeps instead of seeking.
- The engine detects hard disk vs SSD (Windows: the device's seek-penalty property). On a hard disk, checks run one at a time per disk and seeding reads are batched. On an SSD, reads run in parallel.
- Your "max active checking torrents" setting is kept. On a hard disk with a value above 1, Settings shows a one-line note on what it costs.

### 4. Open files only when needed
- Files are opened when a peer asks for a piece from them, kept in a pool of recently used handles (same default size as qBittorrent), and closed when idle. 735 torrents never means 2,063 open files.

### 5. Background I/O priority
- Hash checks, seeding reads and resume writes run at the OS's background I/O priority (Windows thread background mode, Linux idle I/O class, macOS throttled I/O policy), so your games, builds and video calls keep the disk first.
- Hash checks read sequentially and tell the OS not to keep the data in its cache, so checking 1.5 TB doesn't push everything else out of RAM.

### 6. Write little, write together
- Resume data is saved only for torrents that changed, in one SQLite transaction (WAL mode), not 735 files rewritten on a timer.
- No memory-mapped I/O (see `PLAN.md`).

## The first screen

Target: the full list visible within 300 ms of launch, before the engine has started a single torrent.

1. **Snapshot first.** On quit and every few minutes, the engine writes a list snapshot to SQLite: name, size, category, tags, last known progress, state, ratio, added date. At launch the window draws the list from it immediately: sorted, filtered and counted as you left it.
2. **Only visible rows exist.** The list is virtual; with 735 torrents it builds the 20 to 30 rows on screen. Sidebar counts come from one SQL query.
3. **Rows come alive in waves.** Not-yet-started torrents show "Starting" in a muted style with their last known progress. As each wave starts, its rows switch to live state with a short fade. Nothing jumps: row order holds while starting, and the selection stays put.
4. **One quiet progress line.** The status bar shows "Starting torrents 240 of 735" with a thin bar, then disappears. No spinner over the list, no modal.
5. **Details on demand.** The details panel loads only for the selected torrent; the piece map comes from the stored bitfield.
6. **Skeleton only when there is no snapshot.** The very first launch (or after an import) shows skeleton rows shaped like the real ones for the fraction of a second it takes to read the database.
7. **Importing from qBittorrent** shows a small sheet: "Importing 735 torrents from qBittorrent. No recheck needed." with a count and Cancel. The app is usable while it runs.

The mockup shows this: press **Replay startup** (`docs/design-directions-2.html`).

## Budgets

| Measure, 735 torrents, warm start | qBittorrent 5.2.4 | tTorrent target |
|---|---|---|
| List on screen | to be measured | under 300 ms |
| All torrents running | to be measured | under 60 s |
| Disk read in the first 60 s, excluding pieces peers requested | to be measured | under 50 MB (resume data and metadata only) |
| Full hash checks when no file changed | to be measured | 0 |
| Files open at peak | to be measured | file-pool size (qBittorrent's default) |
| Writes in the first 60 s | to be measured | under 5 MB |

Fill the qBittorrent column with `scripts/qb-startup-io.ps1`: quit qBittorrent, run the script, then start qBittorrent.
