# tTorrent

A fast, modern torrent client with a polished look and nothing in the way. Custom title bar, themes with any accent colour, three densities, interface scaling, and a status bar and toolbar you choose the contents of.

## Look

- **Themes:** Follow system, Light, Paper, Sand, Linen, Dark, Gray, Nord, Noctua, Vampire, Ink.
- **Accent:** eight presets or any hex colour. tTorrent adjusts the colour's lightness until it reaches 3:1 contrast on the theme, and shows the measured ratio.
- **Density:** Dense, Compact, Standard. **Interface size:** 80 to 150 %.
- **Status bar:** right-click to choose what it shows, including the memory tTorrent uses.
- **Toolbar:** right-click › Show buttons.

All of these are in Tools › Options › Behavior › Look.

## Build (Windows)

```powershell
powershell -ExecutionPolicy Bypass -File ttorrent\scripts\setup.ps1   # Qt 6.8, vcpkg, libtorrent
powershell -ExecutionPolicy Bypass -File ttorrent\scripts\build.ps1   # build\Release\ttorrent.exe
```

Needs Visual Studio Build Tools 2022, CMake and Python (for `aqtinstall`).

## Licence and credits

tTorrent began as a fork of [qBittorrent](https://www.qbittorrent.org/) and is being rebuilt from there to be leaner, faster and cleaner. Thanks to the qBittorrent project and its contributors, whose work it started from, and to [libtorrent](https://www.libtorrent.org/).

tTorrent is GPL: source files GPLv2 or later, binaries GPLv3 or later (see [COPYING](COPYING) and [COPYING.GPLv3](COPYING.GPLv3)). The custom title bar uses [QWindowKit](https://github.com/stdware/qwindowkit) (Apache-2.0), the UI font is Instrument Sans (OFL), and the Nord theme's colours come from [Nord](https://www.nordtheme.com/) (MIT).
