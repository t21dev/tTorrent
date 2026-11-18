# Opening torrents: file types, magnet links and the Add torrent dialog

What happens when someone double-clicks a `.torrent` file or clicks a magnet link in a browser, on each system. Code: `crates/tt-platform/src/assoc.rs` (defaults), `crates/tt-engine/src/core/adding.rs` (the dialog queue), `crates/tt-ui/src/add_dialog.rs` (the dialog).

## The flow (every system)

1. The system starts `ttorrent ITEM` (a path, a `magnet:` link or an `http(s)` link to a `.torrent`).
2. If tTorrent is already running, that second launch hands the item to it over the local socket (`Request::Open`) and exits (13 ms measured with `tt-bench open-shot`). If not, it becomes the engine.
3. With **Settings › Downloads › Show the Add torrent dialog** on (qBittorrent's "Display torrent content and some options"; on by default), the engine queues the item for the **Add torrent dialog** and opens or brings forward the window ("Bring the dialog to the front"). With it off, the item is added straight away with the Downloads defaults.
4. A `.torrent` is read at once. A magnet link shows "Retrieving metadata…" while librqbit fetches the file list (`list_only`, nothing starts); you can press Add before it arrives. A torrent that is already in the list is not shown again; a notice says so ("its trackers were not merged").
5. The dialog follows qBittorrent's "Add new torrent": save folder with Browse and free space, category (a category with its own folder moves the save path there), tags, Start torrent, Add to top of queue, Skip hash check, stop condition (None, Metadata received, Files checked), content layout (Original, Create subfolder, Don't create subfolder), torrent information (name, size, date, info hashes, comment), the file tree with checkboxes, Select all / Select none, and Never show again. Sequential download and first-and-last-pieces-first are shown greyed: librqbit 9.1 has no call for them.
6. Several items queue up ("Add torrent · 1 of 3"). The queue lives in the engine, so it survives closing the window.

Ctrl+O, File › Add torrent link (one link or info hash per line) and dropping files on the window go through the same dialog.

## Content layout

qBittorrent's three layouts, applied before librqbit sees the torrent (librqbit writes a multi-file torrent straight into its output folder, so tTorrent passes the torrent's own folder):

| Layout | Multi-file torrent "Sintel" | Single file `sintel.mkv` |
|---|---|---|
| Original (default) | `save/Sintel/…` | `save/sintel.mkv` |
| Create subfolder | `save/Sintel/…` | `save/sintel/sintel.mkv` |
| Don't create subfolder | `save/…` | `save/sintel.mkv` |

The Save path column shows `save`, as in qBittorrent. Imports from qBittorrent keep its layout: the root folder comes from the resume data's `mapped_files` (renamed roots and "Don't create subfolder" included).

## Making tTorrent the default

Settings › File association shows whether tTorrent opens `.torrent` files and magnet links today, with a **Make default…** button. What the button can do depends on the system:

| System | What tTorrent writes | Who makes the final choice | How "default?" is read |
|---|---|---|---|
| Windows 10 and 11 | Per user (HKCU, no admin): ProgIDs `tTorrent.File.Torrent` and `tTorrent.Url.Magnet`, `.torrent\OpenWithProgids`, `Software\tTorrent\Capabilities` and `RegisteredApplications`, so tTorrent is listed in Default apps. Where the user never chose an app, it also takes the class default. | The user. Windows protects defaults with a hashed UserChoice that apps can't write, so the button opens **Settings › Default apps** on tTorrent's own page (`ms-settings:defaultapps?registeredAppUser=tTorrent`, Windows 11 from the April 2023 update; older builds open the Default apps list). | `UserChoiceLatest` (24H2) or `UserChoice` ProgId, else the class default |
| macOS 12+ | Nothing at run time: the app bundle's Info.plist declares both (below). | `NSWorkspace setDefaultApplicationAtURL:toOpenURLsWithScheme:` and `…toOpenContentType:` set both; macOS may ask the user to confirm. A bare binary outside an `.app` can't be a handler, and the button says so. | `URLForApplicationToOpenURL` and `URLForApplicationToOpenContentType` |
| Linux | `~/.local/share/applications/ttorrent.desktop` (`Exec="…" %U`, `MimeType=application/x-bittorrent;x-scheme-handler/magnet;`) and its icon; types nobody handles yet are taken. | tTorrent, with `xdg-mime default ttorrent.desktop TYPE` (one type per call), or `mimeapps.list` directly when `xdg-mime` is missing. Inside Flatpak the desktop's settings decide. | `xdg-mime query default TYPE`, else `mimeapps.list` |

qBittorrent does the same on Windows (its Options page shows a button that opens Default apps; its installer writes the Capabilities keys, machine-wide). Neither asks at first start; tTorrent never changes a choice the user made.

`ttorrent --register` runs the same "Make default" from the command line and prints the result.

## macOS: what the bundle needs (Phase 5)

macOS delivers files and links to an app through Apple Events, not as arguments. The engine (`ttorrent`, the bundle's main executable) will run the AppKit loop the tray needs anyway and pass `application:openURLs:` to the same `Request::Open`. Info.plist entries:

- `CFBundleURLTypes`: scheme `magnet`, role Viewer.
- `CFBundleDocumentTypes`: content type `org.bittorrent.torrent`, role Viewer, rank Owner.
- `UTImportedTypeDeclarations`: `org.bittorrent.torrent`, conforms to `public.data`, extension `torrent`, MIME `application/x-bittorrent`.

## Status

| | Windows | macOS | Linux |
|---|---|---|---|
| Register, status, make default | built, run on Windows 11 | built, type-checked only (`cargo check --target aarch64-apple-darwin`), not run | built, type-checked only, not run |
| Open from the system into the dialog | run (`tt-bench open-shot`) | needs the bundle and Apple Events (Phase 5) | same code as Windows (`%U` arguments), not run |
| Add torrent dialog | run, screenshot in the README | same GPUI code, not run | same GPUI code, not run |
