# tTorrent design

How tTorrent looks. New UI follows it; if the code and this file disagree, fix one of them on purpose in the same change. Chosen 18 November 2026: **variant A, Quiet** of the window mock-up (https://claude.ai/artifact/UKsxeqvLk4U9Diyz8eqj1N), refined since with the owner's feedback (below).

Source of truth in code: `src/gui/dose/` (`dosetheme.*` for tokens, accent and density; `theme/themes/*.json`; `theme/stylesheet.qss`, a token template with no colour literals; `dosepaint.*` for what is painted in code).

## Principles

- **Familiar map, new finish.** Menu, toolbar, filter sidebar, transfer list, details tabs and status bar stay where people expect them.
- **Data is the loudest thing.** Chrome is quiet greys. Colour is for the accent (selection, primary action, progress), states (seeding, checking, error) and category colours.
- **Crisp and compact, never gimmicky.** Flat surfaces; depth comes from a lighter top edge, a darker bottom edge and fills, **never gradients**. Icon buttons and icon labels wherever they fit. Every interactive element has a hover state.
- **Fast is a feature.** Nothing decorative costs a frame or memory. No blur, no idle redraws. Lists with hundreds of rows are painted in code, not through stylesheet rules (the stylesheet as a whole costs about 29 MB of private bytes, measured 2026-11-18).
- **Plain copy.** Sentence case, short sentences. No invented numbers.

## Colour

17 themes, Kitab-style (`../kitab/DESIGN.md` › Colour), plus Follow system and the user's own from the theme editor:

- Light: Light, Paper, Latte, Solarized, Blossom
- Dark: Dark, Ink, Charcoal, Nord, Dracula, Mocha, Tokyo, Rosé Pine, Gruvbox, Vampire, Aurora, Ember

Base tokens: `bg`, `panel`, `raised`, `hover`, `line`, `text`, `muted`, `faint`, `ring`, `accent`, `ok`, `warn`, `bad`.

Derived per theme:

| Token | Formula | Use |
|---|---|---|
| `accent` | the preset or custom hex, lightness adjusted in OKLCH to 3:1 on `bg` | Primary button, progress, selection bar, tab underline, focus |
| `select` | accent over `bg`: 14 % light themes, 22 % dark | Selected rows and filters |
| `accentLine` | accent over `bg`, 50 % | Partial pieces, focus borders |
| `track` | `text` over `bg`: 9 % light, 14 % dark | Progress tracks, empty pieces |
| `rowRule` | `line` over `bg`, 60 % | Row dividers |
| `onAccent` | `bg` or `text`, whichever contrasts more with `accent` | Text and icons on accent fills |
| `edgeHi`, `edgeLo` | lighter and darker `line` | Top and bottom edges of raised surfaces |
| `well`, `wellEdge` | darker than `bg` | Text fields and other inset surfaces |
| `card` | between `bg` and `panel` | Settings groups |
| `menuHover` | `hover` tuned for menus | Menu item hover |

Accent presets: Theme (the theme's own), Emerald, Blue, Grape, Amber, Rose, Wine, Mono, or any hex.

Torrent states (status text and icon): downloading, forced, metadata and moving use `accent`; seeding and completed `ok`; stalled and checking `warn`; errors and missing files `bad`; stopped and queued `faint`.

Progress bars use `accent`, `bad` on error or missing files, `muted` when stopped. They follow the state's text colour only when qBittorrent's "progress bar follows text colour" option is on.

## Type

| Role | Face | Size |
|---|---|---|
| UI | Segoe UI Variable (Windows' own), system hinting | 13 px (12 Dense, 14 Standard) |
| Numbers | Segoe UI Variable (its figures are tabular) | UI size; every size, speed, ratio, count, ETA, IP, date and key cap |
| Headings (dialogs) | Segoe UI Variable semibold | 15 to 16 px |

Settings › Look › Font can switch to the bundled Instrument Sans with DM Mono numbers (unhinted, numbers 1 px smaller); those fonts load only when chosen. Default changed on 2026-11-18 at the owner's request: Instrument Sans looked choppy on Windows.

Table headers and labels are sentence case, never tracked capitals.

## Density

| | Dense | Compact (default) | Standard |
|---|---|---|---|
| UI text | 12 | 13 | 14 |
| Control padding (vertical) | 3 | 4 | 7 |
| Row padding (vertical) | 3 | 4 | 7 |
| Radius | 7 | 8 | 10 |
| Layout margins | 6 | 9 | 12 |
| Layout spacing | 4 | 6 | 8 |
| Small icons | 14 | 14 | 16 |

Density changes spacing everywhere, through the stylesheet tokens and `DoseStyle::pixelMetric`. Interface size 80 to 150 %, applied at the next start. Toolbar: Slim (default) or Regular, and icons Small, Medium or Large.

## Components

- **Title bar** (custom on every window and dialog, never the system one): the app icon, the menus in `muted`, the filter field centred, window buttons; Close turns red on hover. Dialogs show their title on the left unless they draw their own heading.
- **Toolbar** (`bg`, hairline below): "Add torrent" is the one primary button (accent fill, `onAccent` icon and label); every other button is an icon. Right-click: icon size, height, which buttons show, reset.
- **Filter field** (in the title bar): a rounded well with a search button that opens "Filter by" and the shortcut in the placeholder.
- **Main tabs** (Transfers, RSS, Log): icon and label, accent underline; the bar hides when Transfers is the only tab, and right-clicking it shows or hides the others.
- **Sidebar** (`panel`): Views (saved filters), Status, Categories, Tags, Trackers; counts in DM Mono `faint`, right-aligned. The selected filter has a `select` fill and an accent icon.
- **Transfer list**: hairline row dividers (`rowRule`), no zebra stripes, rows painted in code. Progress in one of three styles (Settings › Look): **Line** (4 px), **Bar** (8 px), each with the percentage beside it, or **Filled** (the whole cell with the percentage inside, the text switching colour over the fill). Zero speeds and ∞ ETAs fade to `faint`. The selected row gets a `select` fill and a 3 px accent bar on its left edge.
- **Details tabs**: icon and label, an accent underline under the current tab.
- **General**: the **piece map** (the signature): 2 px-rounded squares, done in `accent`, partial in `accentLine`, missing in `track`.
- **Speed graph**: smooth lines with a fading area fill (accent and `ok`), dashed grid, axis labels in DM Mono `faint`.
- **Status bar** (`panel`): information on the left (free space, DHT, memory), connection and speeds on the right (download `accent`, upload `ok`). Right-click chooses what shows. Clicking Memory opens the **memory card**: now, peak, reserved, running time and a graph since start.
- **Buttons**: flat `raised` fill, 1 px `line` border with a darker bottom edge; pressed sinks to `well`. The default button is accent-filled. Disabled buttons are `panel` with `faint` text. Icon buttons (Select all / none, card close) are square and transparent until hovered.
- **Settings**: a sidebar of sections with 15 px icons; groups as `card` boxes with the title above, 12 px radius. Long help text wraps; nothing scrolls sideways.
- **Cards** (notifications, low disk space): `raised`, 12 px radius, bottom-right of the window, an icon, a bold title, a `muted` line and up to two action buttons. They fade in and leave after 8 s unless hovered.
- **Command palette** (Ctrl+K): a floating `panel` card under the title bar; a search well, rows with icon, name, menu name in `faint` and key caps.
- **Key caps**: DM Mono on `well`, 1 px `line` with a 2 px bottom edge, 6 px radius.
- **Tooltips**: `raised`, 1 px `line`, 9 px radius.
- **Menus**: translucent window, `raised`, 10 px radius, compact items with icons and right-aligned shortcuts; menus longer than 16 items get tighter rows.
- **Warning banners**: text in `warn` on a 12 % `warn` wash with a 35 % `warn` border, 8 px radius.

## Icons

- **UI icons:** one set, Lucide geometry (ISC), 24 grid drawn at 14 to 16 px, 1.6 px stroke, round caps and joins. Icons are tinted from tokens when the theme loads, never stored in a colour: chrome icons in `muted`, state icons in their state colour, icons on accent in `onAccent`. Every command has an icon, including in the palette and the shortcuts sheet.
- **App icon:** the sprout-arrow in white on a `#10A874` tile with rounded corners (`ttorrent/brand/icon.svg`; `make_icon.py` draws the PNG and the multi-size ICO). The same icon is used in the title bar and the About dialog, whatever the theme. The tray icon can be the colour tile, outline or filled monochrome.

## Motion

- **Hover and press fade** on every button, tool button, flat button, status bar button, icon button and Add torrent: 120 ms in, 180 ms out, ease-out (`dosemotion.cpp`). The stylesheet keeps size, padding and text colour; the background and border are painted underneath from the same tokens. At rest and at full hover the pixels match the stylesheet's old look.
- **Progress glides** to its new value over 400 ms (ease-out cubic) in the transfer list, for rows on screen only; the percentage text always shows the exact value (`dosepaint.cpp`).
- **Windows fade in:** dialogs over 180 ms; menus, drop-down lists and tooltips over 120 ms; notification cards over 160 ms (`dosestyle.cpp`, `doseautomation.cpp`). Tabs switch at once. Only opacity and colour animate.
- **Nothing animates while idle.** Animations run only while something changes, and Windows' "Animation effects" setting off makes every change instant.
- Graphics effects are removed once a fade ends, so text stays crisp.
- **Base style:** Fusion under both light and dark themes (`DoseStyle`), so sizes match whatever the colour scheme.
