# tTorrent design

How tTorrent looks. New UI follows it; if the code and this file disagree, fix one of them on purpose in the same change. Chosen 18 November 2026: **variant A, Quiet** of the window mock-up (https://claude.ai/artifact/UKsxeqvLk4U9Diyz8eqj1N).

Source of truth in code: `src/gui/dose/` (`dosetheme.*` for tokens, accent and density; `theme/themes/*.json`; `theme/stylesheet.qss`, a token template with no colour literals).

## Principles

- **Familiar map, new finish.** Menu, toolbar, filter sidebar, transfer list, details tabs and status bar stay where people expect them.
- **Data is the loudest thing.** Chrome is quiet greys. Colour is for the accent (selection, primary action, progress), states (seeding, checking, error) and category colours.
- **Fast is a feature.** Nothing decorative costs a frame or memory. No blur, no idle redraws.
- **Plain copy.** Sentence case, short sentences. No invented numbers.

## Colour

Themes and tokens are Kitab's (`../kitab/DESIGN.md` › Colour): Follow system, Light, Paper, Sand, Linen, Dark, Gray, Nord, Noctua, Vampire, Ink. Tokens used: `bg`, `panel`, `raised`, `hover`, `line`, `text`, `muted`, `faint`, `ring`, `accent`, `ok`, `warn`, `bad`.

Derived per theme:

| Token | Formula | Use |
|---|---|---|
| `accent` | the preset or custom hex, lightness adjusted in OKLCH to 3:1 on `bg` | Primary button, progress, selection bar, tab underline, focus |
| `select` | accent over `bg`: 14 % light themes, 22 % dark | Selected rows and filters |
| `accentLine` | accent over `bg`, 50 % | Partial pieces, focus borders |
| `track` | `text` over `bg`: 9 % light, 14 % dark | Progress tracks, empty pieces |
| `rowRule` | `line` over `bg`, 60 % | Row dividers |
| `onAccent` | `bg` or `text`, whichever contrasts more with `accent` | Text on the primary button |

Accent presets: Theme (the theme's own), Emerald, Blue, Grape, Amber, Rose, Wine, Mono, or any hex.

Torrent states: downloading, forced, metadata and moving use `accent`; seeding and completed `ok`; stalled and checking `warn`; errors and missing files `bad`; stopped and queued `faint`.

## Type

| Role | Face | Size |
|---|---|---|
| UI | Instrument Sans 400/500/600 | 13 px (12 Dense, 14 Standard) |
| Numbers | DM Mono 400/500, tabular | UI size − 1 px; every size, speed, ratio, count, ETA, IP and date in a table |
| Headings (dialogs) | Instrument Sans 600 | 15 to 16 px |

Table headers and labels are sentence case, never tracked capitals.

## Density

| | Dense | Compact (default) | Standard |
|---|---|---|---|
| Row | 26 | 30 | 36 |
| Control | 26 | 30 | 36 |
| Radius | 7 | 8 | 9 |

Interface size 80 to 150 %, applied at the next start.

## Components (variant A)

- **Title bar** (38 px, `panel`): logo, the menus in `muted`, the title centred in `faint`, window buttons; Close turns red on hover.
- **Toolbar** (`bg`, hairline below): "Add torrent" is the one primary button (accent fill, label); every other button is a 16 px icon at 30 px, labels optional. The filter field sits on the right with its shortcut shown.
- **Sidebar** (`panel`): Status, Categories (each with its colour square), Tags, Trackers; counts in DM Mono `faint`, right-aligned. The selected filter has a `select` fill and an accent icon.
- **Transfer list**: hairline row dividers (`rowRule`), no zebra stripes. Progress is a 4 px pill on `track` with the percentage beside it in DM Mono; its colour follows the state. Status is a 14 px icon plus a word, both in the state colour. Zero speeds and ∞ ETAs fade to `faint`. The selected row gets a `select` fill and a 3 px accent bar on its left edge.
- **Details tabs**: icon and label, an accent underline under the current tab, and the selected torrent's name on the right in `faint`.
- **General**: the **piece map** (the signature): 48 columns of 2 px-rounded squares, done in `accent`, partial in `accentLine`, missing in `track`. An availability bar sits below it and a two-column fact list beside it.
- **Trackers**: DHT, PeX and LSD first, then tiers; status as an outlined pill (`ok` Working, `bad` Not working); errors in `bad`.
- **Peers**: country flag, IP and port in DM Mono, client, and a small progress bar.
- **Content**: checkboxes in the accent, a priority pill, a progress bar per file.
- **Speed graph**: smooth download and upload lines with a fading area fill (accent and `ok`), dashed horizontal grid, axis labels in DM Mono `faint`, a dot on the latest value, a legend with live values and the peak, and period chips (1m, 5m, 30m, 6h, 24h).
- **Status bar** (30 px, `panel`): free space, DHT, memory, then right-aligned connection (a dot), alternative speed limits, download and upload speeds (in `accent` and `ok`, session totals `faint`). Every item has a tooltip; right-click chooses what shows.
- **Tooltips**: `raised`, 1 px `line`, 8 px radius, soft shadow; a title, its shortcut as a key cap, and an optional one-line hint in `muted`.
- **Menus**: `raised`, 12 px radius, 30 px items with icons and right-aligned shortcuts; destructive items in `bad`.

## Icons

One set, Lucide geometry (ISC), 24 grid drawn at 16 px, 1.6 px stroke, round caps and joins. Icons are tinted from tokens when the theme loads, never stored in a colour: chrome icons in `muted`, state icons in their state colour, the primary button's icon in `onAccent`.

## Motion

120 ms for hover and press, 220 ms for tabs and menus. Only opacity and colour animate. Nothing animates while idle.
