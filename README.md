# tTorrent

A torrent client with qBittorrent's features and a modern finish: a custom title bar, themes with any accent colour, three densities, interface scaling, a status bar and toolbar you choose the contents of, and a redesigned Settings and Add Torrent dialog.

tTorrent is built on [qBittorrent](https://www.qbittorrent.org/) 5.3 and [libtorrent](https://www.libtorrent.org/). It keeps qBittorrent's engine, settings, Web API and file formats, and takes in each qBittorrent release (see [UPSTREAM.md](UPSTREAM.md)).

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

Needs Visual Studio Build Tools 2022, CMake and Python (for `aqtinstall`). qBittorrent's own build notes are in [INSTALL](INSTALL) and [README.qBittorrent.md](README.qBittorrent.md).

## Licence and credits

tTorrent is GPL, like qBittorrent: source files GPLv2 or later, binaries GPLv3 or later (see [COPYING](COPYING) and [COPYING.GPLv3](COPYING.GPLv3)). It's a fork of qBittorrent by the qBittorrent project and its contributors. The custom title bar uses [QWindowKit](https://github.com/stdware/qwindowkit) (Apache-2.0), the UI font is Instrument Sans (OFL), and the Nord theme's colours come from [Nord](https://www.nordtheme.com/) (MIT).
