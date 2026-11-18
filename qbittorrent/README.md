# Dose for qBittorrent

A theme that gives stock qBittorrent (5.x, Windows, macOS, Linux) tTorrent's Dose look: neutral surfaces, green accent, Instrument Sans, hairline rows, soft selection, rounded inputs and menus, and status colours for the transfer list. Light and dark both work from the same theme.

## Install

1. Optional: install the fonts in `dose-theme/fonts/` (Windows: right-click › Install). Without them it falls back to Segoe UI.
2. qBittorrent › Tools › Options › Behavior › Interface › tick **Use custom UI Theme** and pick `dose-theme/config.json`.
3. Restart qBittorrent.

## Rebuild

`python qbittorrent/build_theme.py` regenerates `dose-theme/` from the Dose tokens in DESIGN.md.

## Status (18 Nov 2026)

- Built: palette (light and dark), stylesheet, 63 icons in two colours, fonts.
- Not yet checked in a running qBittorrent: no screenshots taken yet.
- Not started: the tuned-settings profile and its measurements against defaults, and the MCP companion that talks to qBittorrent's Web API.
