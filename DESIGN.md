# tTorrent design

How tTorrent looks and behaves. New UI must follow it. If the code and this file disagree, fix one of them on purpose, in the same change.

Source of truth in code (once it exists): `crates/tt-theme` (tokens, themes, density, scale, accent maths) and `assets/themes/*.json`. The reference mockup is `docs/design-directions-2.html` (direction **D. Dose**), published at https://claude.ai/artifact/YBQ5AQ9jyYud5i7kA5nNof.

## Principles

- **Familiar first.** The day-to-day layout is qBittorrent's: menu, toolbar, filter sidebar, transfer list, details tabs, status bar. People switching from qBittorrent find every action where they expect it. We change the finish, not the map.
- **Data is the loudest thing.** The transfer list carries the screen. Chrome is quiet greys. Colour is for the accent (selection, primary action, progress), status (seeding, error, warning) and category colours.
- **Small, calm controls.** Toolbar buttons are icon-only by default at 28 px. Labels are a per-button choice, never forced.
- **Everything is the user's choice.** Toolbar items, order and label mode, status bar items, details tabs, density, scale, theme, tint, accent and app icon are all settings. Defaults are good; nothing is locked.
- **Motion is state, never decoration.** Transitions explain a change (a button appears, a tab moves, a row is selected). Every one can be turned off, and all of them stop under the system's reduced-motion setting.
- **Fast is a feature.** Nothing decorative may cost a frame or memory. No blur, no animated backgrounds, no idle redraws.
- **Plain copy.** Sentence case, short sentences. No invented numbers anywhere, in the app or on the site.

## Themes

One direction, **Dose**, in two modes (light, dark) and two surface tints:

- **Neutral** (default): greys with hue 260 and chroma at or below 0.006. No colour cast.
- **Tinted**: the original warm Dose paper (hue 88 to 165).

Each colour is derived again for dark mode, never inverted: paper L 15 to 21 %, ink L 94 to 95 %, accents +20 % lightness and slightly lower chroma. The hue never moves between modes.

### Tokens: neutral (default)

| Token | Light | Dark | Use |
|---|---|---|---|
| `bg` | `oklch(98.7% 0.002 260)` | `oklch(17% 0.003 260)` | Transfer list, window |
| `side` | `oklch(96.6% 0.003 260)` | `oklch(19.5% 0.004 260)` | Filter sidebar, title bar |
| `panel` | `oklch(99.5% 0.001 260)` | `oklch(20.5% 0.004 260)` | Toolbar, details, status bar |
| `raise` | `oklch(99.7% 0.001 260)` | `oklch(24% 0.005 260)` | Menus, sheets, toasts, buttons |
| `well` | `oklch(96.4% 0.003 260)` | `oklch(15.5% 0.003 260)` | Inputs, search |
| `ink` | `oklch(20% 0.006 260)` | `oklch(94.5% 0.003 260)` | Primary text |
| `ink-2` | `oklch(30% 0.006 260)` | `oklch(85% 0.004 260)` | Table text |
| `muted` | `oklch(48% 0.008 260)` | `oklch(70% 0.006 260)` | Headers, counts, helper text |
| `rule` | `oklch(88.5% 0.004 260)` | `oklch(30% 0.005 260)` | Pane hairlines |
| `row-rule` | `oklch(93.5% 0.003 260)` | `oklch(23% 0.004 260)` | Row dividers |
| `hover` | `oklch(95.2% 0.004 260)` | `oklch(23.5% 0.005 260)` | Hover fill |
| `track` | `oklch(92% 0.004 260)` | `oklch(28% 0.005 260)` | Progress tracks, empty pieces |
| `ok` | `oklch(52% 0.12 150)` | `oklch(76% 0.12 150)` | Seeding, completed |
| `warn` | `oklch(58% 0.13 65)` | `oklch(80% 0.12 70)` | Checking, stalled |
| `bad` | `oklch(55% 0.18 25)` | `oklch(72% 0.15 25)` | Errors, destructive actions |

The tinted set is in the mockup's `[data-scheme="ds"]` blocks; it is the same table with warm hues.

### Accent

Presets (light / dark): Green `oklch(50% 0.13 160)` / `oklch(70% 0.11 160)` (default), Blue, Violet, Amber, Rose, Ink. A custom colour keeps its hue and chroma; its lightness is adjusted per mode until it reaches 3:1 against `bg` for marks and 4.5:1 where it is text (Kitab's rule). Derived: `accent-soft` = accent at 15 % (selection fill), `accent-line` = 50 % (partial pieces, focus borders). Accent changes animate over 420 ms.

The accent marks the primary button, the selected row (3 px inset bar plus soft fill), the selected filter, progress fills, the tab underline, focus rings and switches. Never large fills, never body text.

## Type

| Role | Face | Size |
|---|---|---|
| UI | Instrument Sans 400/500/600 | 13 px (12.5 compact, 13.5 comfortable) |
| Numbers | DM Mono 400/500, tabular | 12 px in tables |
| Heads (sheets, brand) | Schibsted Grotesk 700, −0.03 em | 15 to 17 px |

Users may pick any installed font for the UI (Settings › Appearance › Interface font) and a separate number font. Table headers and labels are sentence case, never tracked capitals.

## Density and scale

| | Compact | Cozy (default) | Comfortable |
|---|---|---|---|
| Row | 26 px | 34 px | 42 px |
| Toolbar | 38 px | 44 px | 50 px |
| Button | 24 px | 28 px | 32 px |
| Sidebar | 196 px | 216 px | 236 px |

UI scale: 80, 90, 100, 110, 125, 150 %, independent of the OS scale. Ctrl/⌘ `=` `-` `0`.

## Shape and depth

Radius: window 12, buttons and inputs 10, menu items 8, menus and sheets 12, toasts 12, progress bars pill, pieces 2. Buttons are moulded: a 1 px top highlight and a 1 px bottom edge, pressing down 1 px. Menus, sheets and toasts float with a two-layer shadow. Panes are separated by 1 px hairlines, not by shadows.

## Layout

```
Windows:  [title bar: icon · File Edit View Torrent Tools Help · title · – □ ×]
          [toolbar                                              ]
          [sidebar | transfer list                              ]
          [        | details tabs + pane                        ]
          [status bar                                           ]

macOS:    system menu bar: tTorrent File Edit View Torrent Tools Window Help
          [traffic lights over a full-height sidebar | toolbar  ]
          [sidebar                                  | list      ]
          [                                         | details   ]
          [                                         | status bar]
```

- **Windows** uses its native title bar with the menu inside it. **macOS** uses the system menu bar and a unified title bar with the traffic lights over the sidebar. **Linux** follows Windows.
- **Sidebar** (Ctrl/⌘ B): Status, Categories (each with its colour square), Tags, Trackers, each item with a count.
- **Transfer list** columns default to: Name, Added on, Size, Progress, Status, Downloaded, Uploaded, Down, Up, ETA, Ratio. All **38 qBittorrent columns** are available, in qBittorrent's order (from `transferlistmodel.h`): #, Name, Size, Total size, Progress, Status, Seeds, Peers, Down speed, Up speed, ETA, Ratio, Popularity, Category, Tags, Added on, Completed on, Tracker, Down limit, Up limit, Downloaded, Uploaded, Session download, Session upload, Remaining, Time active, Save path, Completed, Ratio limit, Last seen complete, Last activity, Availability, Incomplete save path, Info hash v1, Info hash v2, Reannounce in, Private, Created on. Right-click a header (or View › Columns…) to show or hide them; a shown column is inserted at its qBittorrent position and fades in. Drag headers to reorder, drag their edges to resize. Click a header to sort (arrow shows direction; numbers sort descending first, text ascending). Default sort: Added on, newest first. Name always shows. Numbers right-aligned in DM Mono.
- **Details** (Ctrl/⌘ J): General (the live piece map plus key figures), Trackers, Peers, HTTP sources, Content, Speed. The tab underline slides between tabs.
- **Status bar**: free space, DHT nodes, down and up speed with session totals, memory against its cap, port state. Optional: session ratio, connected peers.

## Torrent menu (right-click on a row)

Same items, order, submenus and icons as qBittorrent's, so muscle memory carries over. Opens with right-click, Shift+F10 or the menu key; arrows move, → opens a submenu, ← closes it, Esc closes and returns focus to the list.

```
Stop / Start                 Ctrl+P / Ctrl+S   (label follows the state)
Force start
──────────────────────────
Remove                       Delete            (icon in the bad colour)
──────────────────────────
  Set location…
  Rename…                    F2
  Manage content…
  Edit trackers…
  Category                ›  New category…, Reset, then each category with a check
  Tags                    ›  Add tags…, Remove all, then each tag with a check
  Automatic torrent management   (check)
──────────────────────────
  Queue                   ›  Move to top, Move up, Move down, Move to bottom (when queueing is on)
  Torrent options…
  Super seeding mode             (check)
──────────────────────────
  Preview file…
──────────────────────────
  Force recheck              Ctrl+R
  Force reannounce
──────────────────────────
  Open destination folder    Ctrl+Enter
  Copy                    ›  Name, Info hash v1, Info hash v2, Magnet link, Torrent ID, Comment
  Export .torrent…
```

With several rows selected, the menu acts on all of them. Submenus open on hover or →, and flip to the left at the window edge.

## Tray and menu-bar icon

Same items, order and shortcuts as qBittorrent's tray menu so it feels identical, plus a live header and two additions (marked new):

```
  ↓ 4.1 MB/s   ↑ 1.6 MB/s   12 active            (new: live, not clickable)
  ─────────────────────────────────────────
  Show tTorrent / Hide                            (toggles with the window)
  ─────────────────────────────────────────
     Add torrent file…                Ctrl+O
     Add magnet link…                 Ctrl+Shift+O
  ─────────────────────────────────────────
     Slow mode (alternative speed limits)   ✓
     Set global speed limits…
  ─────────────────────────────────────────
     Pause all / Resume all           Ctrl+Shift+P
     Pause for 1 hour                         (new)
  ─────────────────────────────────────────
     Exit                             Ctrl+Q
```

- Left-click shows or hides the window (Windows, Linux); on macOS the menu-bar icon opens the menu.
- The tray icon is the chosen app icon, monochrome on macOS. While downloading it shows a small progress ring; in slow mode, a small dot in the warn colour.
- The tooltip shows speeds and active count. Hovering the taskbar icon shows overall progress (Windows taskbar progress, macOS Dock badge).
- The tray lives in the engine process, so it costs no GPU memory when the window is closed (see PLAN.md, architecture).

## Signature

The **piece map** in the General tab: 48 columns of small squares, filled in the accent as pieces arrive, each new piece briefly scaling up. It is the one picture of what a torrent is.

## Customization

- **Toolbar**: 15 available items, drag to reorder, each set to Hidden, Icon only, or Icon and label using three icon buttons. From right-click on the toolbar, View › Customize toolbar, or Settings › Toolbar. Keyboard: Alt and the arrow keys move a focused item.
- **Status bar** and **details tabs**: right-click to show or hide items (at least one tab stays).
- Items appear and leave with a 220 ms fade and scale; neighbours slide into place (transform only).
- **Settings** has a search field (focused on open, Ctrl/⌘ F) that filters rows and sections live and highlights matches, plus a section list.

## AI agents

Settings › AI agents follows Turbo Reader's page: an on/off switch (off by default), the address, API keys (name, scope, last used; shown once), ready-to-copy connect commands per agent, and **Activity**. When an agent wants to delete files or change more than 20 torrents, tTorrent shows a confirmation sheet naming the agent and the torrents, with Allow and Deny. Agent actions appear in the status bar briefly ("Claude Code paused 3 torrents") with Undo. Spec: `docs/mcp.md`.

## App icon and logo

The logo is **Sprout arrow**: a solid circle with a sprout cut out, its stem ending in a download arrowhead. Grows up, downloads down. The default app icon is Sprout arrow on the light cream tile (`oklch(97% 0.012 90)`) with an ink circle (`oklch(19% 0.012 160)`).

Users choose the app icon in Settings › App icon: Sprout arrow (green, dark, light), Arrow tile, Rooted, Falling seed, Notched badge, and one that follows the accent. The choice changes the taskbar or Dock icon, the tray icon and the title-bar icon. Marks are drawn on a 64 px grid so they stay sharp at 16 px.

## Icons

One set, 16 px on a 16 px grid, 1.6 px stroke, round caps and joins, open shapes. Every icon button has a tooltip with its shortcut and an accessible label.

## Motion

| Token | Value | Use |
|---|---|---|
| `dur-micro` | 120 ms | Hover, press |
| `dur-short` | 220 ms | Tabs, items appearing, menus, rows entering |
| `dur-long` | 420 ms | Mode and accent changes, progress fills, sheets |
| `ease-out` | `cubic-bezier(0.16, 1, 0.3, 1)` | Default |
| `ease-spring` | `cubic-bezier(0.34, 1.56, 0.64, 1)` | Switch knobs, sheet entrance, piece pop |

Only transform, opacity and colour animate. Nothing animates while idle. A frame is drawn only when something changed.

## Keyboard

Shortcuts follow the OS: **⌘ on macOS, Ctrl on Windows and Linux**, everywhere, including the command palette and search. Displayed with ⌘ ⇧ ⌥ ⌫ ↩ on macOS. qBittorrent's shortcuts are kept where it has one.

| Action | Windows / Linux | macOS |
|---|---|---|
| Add torrent file | Ctrl+O | ⌘O |
| Add magnet link | Ctrl+Shift+O | ⇧⌘O |
| Create torrent | Ctrl+N | ⌘N |
| Resume / Pause | Ctrl+S / Ctrl+P | ⌘S / ⌘P |
| Pause or resume selected | Space | Space |
| Pause all / resume all (session) | Ctrl+Shift+P | ⇧⌘P |
| Remove / with files | Delete / Shift+Delete | ⌫ / ⇧⌫ |
| Rename | F2 | F2 |
| Open containing folder | Ctrl+Enter | ⌘↩ |
| Copy magnet link | Ctrl+Shift+C | ⇧⌘C |
| Force recheck | Ctrl+R | ⌘R |
| Select all | Ctrl+A | ⌘A |
| Filter torrents / Command palette | Ctrl+F / Ctrl+K | ⌘F / ⌘K |
| Sidebar / Details panel | Ctrl+B / Ctrl+J | ⌘B / ⌘J |
| Details tab 1 to 6 | Ctrl+1 … 6 | ⌘1 … 6 |
| Zoom in / out / actual size | Ctrl+= / Ctrl+- / Ctrl+0 | ⌘= / ⌘- / ⌘0 |
| Slow mode | Ctrl+Alt+L | ⌥⌘L |
| Settings | Ctrl+, | ⌘, |
| Close window (keeps running) | Ctrl+W | ⌘W |
| Quit | Ctrl+Q | ⌘Q |
| Shortcuts sheet | ? | ? |
| Open the menu bar | Alt or F10 | (system) |

Navigation: ↑ ↓ Home End move the selection; Enter moves to the details tabs; ← → switch tabs; Tab moves between toolbar, sidebar, list and details; Enter or Space on a column header sorts it; Esc closes a menu or sheet and clears a search first.

## States

Every region that loads data has four states: loading (a skeleton in the shape of the rows), empty (what belongs here and the one action that fills it: "Drop a .torrent file anywhere, or press Ctrl+O"), error (what broke and what to do, e.g. "Disk full: free space or move the torrent"), and full. Destructive actions confirm or offer Undo in a toast.

## Accessibility

Contrast 4.5:1 for text and 3:1 for marks in every theme, mode and accent. Every control is reachable by keyboard with a visible 2 px accent focus ring. The list is a grid with row selection announced. Reduced motion turns every transition into an instant change.
