# When files go missing

Torrents outlive the files and folders they point at: someone deletes a download, unplugs a drive, renames a folder, or an antivirus locks a file. tTorrent tells these cases apart, never destroys data or re-downloads terabytes on its own, and offers one clear fix for each.

Status: specification for Phase 1 (engine) and Phase 2 (window).

## Rules

1. **Never assume deletion when a drive is just absent.** Check the volume first.
2. **Never re-download completed content on its own.** Missing content waits for your choice.
3. **Never run a full recheck to find out.** Use sizes and modification times, then hash only the pieces of the affected files.
4. **Keep the torrent's history** (ratio, seeding time, added date, category, tags) through every fix.
5. **One notice per cause, not one per torrent.** A missing drive with 120 torrents on it is one message.

## Detection

- **At startup**: one metadata pass (sizes, modification times, existence), as in `docs/startup.md`. No file is opened.
- **While running**: the save roots (category paths, the default path, the incomplete folder) are watched for deletes and renames. The OS volume events (drive added, removed or locked) are watched too.
- **On use**: a read or write error when a peer asks for a piece triggers a check of that file only.
- **Periodic**: a low-priority metadata pass over seeding torrents every 30 minutes (setting), to catch changes made while nothing was reading.

## Cases

| # | What happened | How tTorrent tells | State shown | What it does | Your options |
|---|---|---|---|---|---|
| 1 | **The drive is gone**: USB or external disk unplugged, network share offline, BitLocker locked, drive letter changed | The volume itself is missing or locked, or a different volume (by serial number) is at that letter | **Drive unavailable** (warn colour) | Pauses those torrents; when the same volume returns, checks sizes and times and resumes them without a recheck | Wait; Locate (if the drive got a new letter) |
| 2 | **All content deleted** | Volume present, the torrent's folder or file is gone | **Missing files** (bad colour) | Stops the torrent; keeps everything else about it | Locate…; Download again; Remove torrent |
| 3 | **Some files deleted** | Some files gone, others intact | **Missing files (3 of 12)** | Keeps seeding the intact files; marks the missing ones in the Content tab and their pieces in the piece map | Download missing files; Locate…; Skip those files |
| 4 | **Files moved or renamed** by you | Missing at the old path; files with the same sizes found nearby | Missing files, with "Found 3 files that look like these" | Nothing until you agree | Use these files (checks a few pieces of each, then resumes); Locate… |
| 5 | **Files changed or truncated** by another app | Size or modification time differs | **Checking** the affected files only | Hashes only those files' pieces, at background priority; re-downloads only bad pieces if the torrent is still downloading | — |
| 6 | **Can't access**: file locked by another app or antivirus, permission denied, read-only | Open or read fails with an access error, files exist | **Can't access** (warn colour) | Retries with back-off (10 s, 1 min, 5 min); explains which file and why | Retry now; Open folder |
| 7 | **Disk full** | Write fails for lack of space | **Disk full** (bad colour) | Pauses downloads on that disk (seeding continues); resumes when space is free | Free space; Move torrent… |
| 8 | **Save folder deleted while downloading** | Folder gone, volume present, torrent not finished | Downloading | Recreates the folder and continues (nothing finished is lost) | — |
| 9 | **Incomplete folder, category folder or watched folder deleted** | Folder gone | Settings shows a note on that setting | Recreates incomplete and category folders on next use; pauses the watched folder until it exists | Choose another folder |
| 10 | **The original .torrent file deleted** | — | No change | Nothing: the metadata lives in tTorrent's database, not in that file | — |
| 11 | **Name clash when adding**: files already exist at the destination | Files with the torrent's names exist | Add dialog note | Asks before touching them | Check and use the existing files (resume from what matches); Save elsewhere; Replace |
| 12 | **tTorrent's own database damaged** (power cut, disk error) | Integrity check at start fails | One-time notice | Restores from the newest automatic backup (daily, last 7 kept), then rebuilds anything newer from the torrents' metadata | Show backups |

## In the window

- **Status column**: the states above, with their icons, and the matching row colour on the status text only (never the whole row).
- **Sidebar**: the Errored filter includes Missing files, Can't access and Disk full; Drive unavailable gets its own filter item while it applies, with a count.
- **Details › General**: a callout above the piece map in plain words: what happened, which path, and one primary action plus the others. Example: "The folder D:\Downloads\Films\Elephants Dream is gone. [Locate…] [Download again] [Remove torrent]".
- **One notice per cause**: "Drive E: is unavailable. 120 torrents are paused until it's back." in the status bar and as one desktop notification, with Show.
- **Locate…** opens a folder picker on the parent of the old path. tTorrent then matches files by name and size, checks a sample of pieces from each, and resumes. Many torrents from the same moved folder can be located in one go ("Use this folder for all 34 torrents that were in D:\Old?").
- **Content tab**: missing files are marked and show their size in the "missing" column; "Download missing files" is available there and in the right-click menu.

## For AI agents

`diagnose_torrent` reports these states with their cause and options. `relocate_torrents` (Manage scope) points torrents at a new folder with the same checks as Locate. Re-downloading completed content needs the Manage scope and is shown in the app first when it exceeds 10 GB.

## Testing

`tt-bench` fixtures include each case: a deleted folder, three deleted files, a moved folder, a truncated file, a locked file, a removed volume (a mounted VHD on Windows that the test detaches), a full volume (a small VHD), and a damaged database. Each must reach the right state without a full recheck (counter D2 stays at 0 for torrents whose files didn't change).
