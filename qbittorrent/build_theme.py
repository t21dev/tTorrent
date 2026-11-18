"""Build the Dose theme for qBittorrent (qbittorrent/dose-theme/).

qBittorrent loads a folder theme from Options > Behavior > Interface >
"Use custom UI Theme": pick the theme's config.json. The folder holds
config.json (palette and state colours, light and dark), stylesheet.qss
(shapes and spacing; colours come from the palette, so one stylesheet serves
both modes) and icons/ (icons/dark/ for dark mode), which replace qBittorrent's
icons by name. Format: qBittorrent's src/gui/uithemesource.cpp.

Colours are DESIGN.md's Dose tokens (neutral, green accent), converted from
OKLCH to sRGB. Run: python qbittorrent/build_theme.py
"""

import json
import math
import re
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent
REPO = ROOT.parent
OUT = ROOT / "dose-theme"


def oklch(lightness: float, chroma: float, hue: float) -> str:
    L = lightness / 100
    a = chroma * math.cos(math.radians(hue))
    b = chroma * math.sin(math.radians(hue))
    l_ = L + 0.3963377774 * a + 0.2158037573 * b
    m_ = L - 0.1055613458 * a - 0.0638541728 * b
    s_ = L - 0.0894841775 * a - 1.2914855480 * b
    l, m, s = l_**3, m_**3, s_**3
    rgb = (
        4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
        -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
        -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s,
    )

    def enc(x: float) -> int:
        x = max(0.0, min(1.0, x))
        x = 12.92 * x if x <= 0.0031308 else 1.055 * x ** (1 / 2.4) - 0.055
        return round(x * 255)

    return "#%02X%02X%02X" % tuple(enc(v) for v in rgb)


# DESIGN.md › Themes › Tokens: neutral. (light, dark) as OKLCH.
TOKENS = {
    "bg": ((98.7, 0.002, 260), (17, 0.003, 260)),
    "side": ((96.6, 0.003, 260), (19.5, 0.004, 260)),
    "panel": ((99.5, 0.001, 260), (20.5, 0.004, 260)),
    "raise": ((99.7, 0.001, 260), (24, 0.005, 260)),
    "well": ((96.4, 0.003, 260), (15.5, 0.003, 260)),
    "ink": ((20, 0.006, 260), (94.5, 0.003, 260)),
    "ink2": ((30, 0.006, 260), (85, 0.004, 260)),
    "muted": ((48, 0.008, 260), (70, 0.006, 260)),
    "rule": ((88.5, 0.004, 260), (30, 0.005, 260)),
    "rowrule": ((93.5, 0.003, 260), (23, 0.004, 260)),
    "hover": ((95.2, 0.004, 260), (23.5, 0.005, 260)),
    "track": ((92, 0.004, 260), (28, 0.005, 260)),
    "ok": ((52, 0.12, 150), (76, 0.12, 150)),
    "warn": ((58, 0.13, 65), (80, 0.12, 70)),
    "bad": ((55, 0.18, 25), (72, 0.15, 25)),
    # Green accent, and its soft selection fill (accent at about 15 % over bg).
    "accent": ((50, 0.13, 160), (70, 0.11, 160)),
    "accentsoft": ((93.5, 0.035, 160), (30, 0.045, 160)),
    "accentline": ((76, 0.08, 160), (48, 0.08, 160)),
}


def palette(mode: int) -> dict[str, str]:
    t = {k: oklch(*v[mode]) for k, v in TOKENS.items()}
    return {
        # Chrome (toolbar, tabs, status bar) on the side colour, lists on bg.
        "Palette.Window": t["side"],
        "Palette.WindowText": t["ink"],
        "Palette.Base": t["bg"],
        # The stylesheet uses alternate-base for hover, light for input wells,
        # midlight for row hairlines, mid for pane hairlines, dark for muted text.
        "Palette.AlternateBase": t["hover"],
        "Palette.Text": t["ink"],
        "Palette.ToolTipBase": t["ink"],
        "Palette.ToolTipText": t["bg"],
        "Palette.BrightText": t["bad"],
        "Palette.Highlight": t["accentsoft"],
        "Palette.HighlightedText": t["ink"],
        "Palette.Button": t["raise"],
        "Palette.ButtonText": t["ink"],
        "Palette.Link": t["accent"],
        "Palette.LinkVisited": t["accent"],
        "Palette.Light": t["well"],
        "Palette.Midlight": t["rowrule"],
        "Palette.Mid": t["rule"],
        "Palette.Dark": t["muted"],
        "Palette.Shadow": t["rule"],
        "Palette.WindowTextDisabled": t["muted"],
        "Palette.TextDisabled": t["muted"],
        "Palette.ToolTipTextDisabled": t["muted"],
        "Palette.BrightTextDisabled": t["muted"],
        "Palette.HighlightedTextDisabled": t["muted"],
        "Palette.ButtonTextDisabled": t["muted"],
        # Transfer list states: DESIGN.md › colour is for status.
        "TransferList.Downloading": t["accent"],
        "TransferList.StalledDownloading": t["warn"],
        "TransferList.DownloadingMetadata": t["accent"],
        "TransferList.ForcedDownloadingMetadata": t["accent"],
        "TransferList.ForcedDownloading": t["accent"],
        "TransferList.Uploading": t["ok"],
        "TransferList.StalledUploading": t["ok"],
        "TransferList.ForcedUploading": t["ok"],
        "TransferList.QueuedDownloading": t["muted"],
        "TransferList.QueuedUploading": t["muted"],
        "TransferList.CheckingDownloading": t["warn"],
        "TransferList.CheckingUploading": t["warn"],
        "TransferList.CheckingResumeData": t["warn"],
        "TransferList.StoppedDownloading": t["muted"],
        "TransferList.StoppedUploading": t["ok"],
        "TransferList.Moving": t["accent"],
        "TransferList.MissingFiles": t["bad"],
        "TransferList.Error": t["bad"],
        "PiecesBar.Border": t["rule"],
        "PiecesBar.Piece": t["accent"],
        "PiecesBar.PartialPiece": t["accentline"],
        "PiecesBar.MissingPiece": t["track"],
        "ProgressBar": t["accent"],
        "Log.TimeStamp": t["muted"],
        "Log.Info": t["accent"],
        "Log.Warning": t["warn"],
        "Log.Critical": t["bad"],
        "Log.BannedPeer": t["bad"],
    }


STYLESHEET = """\
/* Dose for qBittorrent. Colours come from the theme's palette (config.json),
   so this one stylesheet serves light and dark. Built by build_theme.py. */

* {
    font-family: "Instrument Sans", "Segoe UI Variable Text", "Segoe UI", sans-serif;
    font-size: 13px;
}

QMainWindow, QDialog, QDockWidget { background: palette(window); }

QToolBar {
    background: palette(window);
    border: none;
    border-bottom: 1px solid palette(mid);
    padding: 6px 8px;
    spacing: 4px;
}
QToolBar::separator { width: 1px; background: palette(mid); margin: 6px 6px; }
QToolButton {
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 5px;
}
QToolButton:hover { background: palette(alternate-base); }
QToolButton:pressed { background: palette(midlight); }
QToolButton:checked { background: palette(highlight); }

QMenuBar { background: palette(window); padding: 2px 4px; }
QMenuBar::item { padding: 5px 10px; border-radius: 6px; background: transparent; }
QMenuBar::item:selected { background: palette(alternate-base); }
QMenu {
    background: palette(button);
    border: 1px solid palette(mid);
    border-radius: 10px;
    padding: 6px;
}
QMenu::item { padding: 6px 28px 6px 10px; border-radius: 6px; }
QMenu::item:selected { background: palette(alternate-base); color: palette(text); }
QMenu::item:disabled { color: palette(dark); }
QMenu::separator { height: 1px; background: palette(mid); margin: 6px 4px; }
QMenu::icon { padding-left: 6px; }

QTreeView, QTableView, QListView, QTreeWidget, QListWidget {
    background: palette(base);
    alternate-background-color: palette(base);
    border: none;
    selection-background-color: palette(highlight);
    selection-color: palette(highlighted-text);
    outline: 0;
}
QTreeView::item, QTableView::item, QListView::item {
    padding: 5px 4px;
    border-bottom: 1px solid palette(midlight);
}
QTreeView::item:hover, QListView::item:hover { background: palette(alternate-base); }
QTreeView::item:selected, QTableView::item:selected, QListView::item:selected {
    background: palette(highlight);
    color: palette(highlighted-text);
}

QHeaderView { background: palette(base); border: none; }
QHeaderView::section {
    background: palette(base);
    color: palette(dark);
    border: none;
    border-bottom: 1px solid palette(mid);
    border-right: 1px solid palette(midlight);
    padding: 6px 8px;
    font-weight: 500;
}
QHeaderView::section:hover { color: palette(text); }

QTabWidget::pane { border: none; border-top: 1px solid palette(mid); }
QTabBar::tab {
    background: transparent;
    color: palette(dark);
    padding: 8px 12px;
    border: none;
    border-bottom: 2px solid transparent;
}
QTabBar::tab:hover { color: palette(text); }
QTabBar::tab:selected { color: palette(text); border-bottom: 2px solid palette(link); }

QLineEdit, QComboBox, QSpinBox, QDoubleSpinBox, QDateTimeEdit, QPlainTextEdit, QTextEdit, QKeySequenceEdit {
    background: palette(light);
    border: 1px solid palette(mid);
    border-radius: 8px;
    padding: 5px 8px;
    selection-background-color: palette(highlight);
    selection-color: palette(highlighted-text);
}
QLineEdit:focus, QComboBox:focus, QSpinBox:focus, QDoubleSpinBox:focus, QPlainTextEdit:focus, QTextEdit:focus {
    border: 1px solid palette(link);
}
QComboBox::drop-down { border: none; width: 22px; }
QComboBox QAbstractItemView {
    background: palette(button);
    border: 1px solid palette(mid);
    selection-background-color: palette(alternate-base);
    selection-color: palette(text);
    padding: 4px;
}

QPushButton {
    background: palette(button);
    border: 1px solid palette(mid);
    border-radius: 8px;
    padding: 6px 14px;
}
QPushButton:hover { background: palette(alternate-base); }
QPushButton:pressed { background: palette(midlight); }
QPushButton:default {
    background: palette(link);
    border-color: palette(link);
    color: palette(base);
    font-weight: 600;
}
QPushButton:disabled { color: palette(dark); }

QGroupBox {
    border: 1px solid palette(mid);
    border-radius: 10px;
    margin-top: 14px;
    padding: 10px 10px 8px 10px;
}
QGroupBox::title {
    subcontrol-origin: margin;
    left: 10px;
    padding: 0 4px;
    font-weight: 600;
}

QProgressBar {
    background: palette(midlight);
    border: none;
    border-radius: 4px;
    text-align: center;
    color: palette(text);
}
QProgressBar::chunk { background: palette(link); border-radius: 4px; }

QScrollBar:vertical { background: transparent; width: 10px; margin: 0; }
QScrollBar:horizontal { background: transparent; height: 10px; margin: 0; }
QScrollBar::handle { background: palette(mid); border-radius: 4px; margin: 2px; }
QScrollBar::handle:vertical { min-height: 28px; }
QScrollBar::handle:horizontal { min-width: 28px; }
QScrollBar::handle:hover { background: palette(dark); }
QScrollBar::add-line, QScrollBar::sub-line { width: 0; height: 0; }
QScrollBar::add-page, QScrollBar::sub-page { background: none; }

QSplitter::handle { background: palette(mid); }
QSplitter::handle:horizontal { width: 1px; }
QSplitter::handle:vertical { height: 1px; }

QStatusBar {
    background: palette(window);
    border-top: 1px solid palette(mid);
    color: palette(dark);
}
QStatusBar::item { border: none; }

QToolTip { border: none; padding: 4px 8px; border-radius: 6px; }
"""

# qBittorrent icon id -> tTorrent icon (assets/icons) or a path drawn here
# (Lucide-style geometry, 24 x 24, stroke only). Icons not listed keep
# qBittorrent's own.
EXTRA = {
    "insert-link": "M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7",
    "go-top": "M5 4h14M12 20V9M7 14l5-5 5 5",
    "go-up": "M12 19V5M6 11l6-6 6 6",
    "go-down": "M12 5v14M6 13l6 6 6-6",
    "go-bottom": "M5 20h14M12 4v11M7 10l5 5 5-5",
    "torrent-start-forced": "M4 5l8 7-8 7zM13 5l8 7-8 7z",
    "application-exit": "M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4M16 17l5-5-5-5M21 12H9",
    "folder-new": "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM12 10v6M9 13h6",
    "edit-clear": "M18 6 6 18M6 6l12 12",
    "filter-inactive": "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM10 9v6M14 9v6",
}
FROM_ASSETS = {
    "list-add": "plus",
    "torrent-magnet": "magnet",
    "list-remove": "trash",
    "torrent-start": "play",
    "torrent-stop": "stop",
    "paused": "pause",
    "stopped": "stop",
    "configure": "sliders",
    "preferences-desktop": "sliders",
    "preferences-advanced": "chip",
    "preferences-bittorrent": "globe",
    "preferences-webui": "globe",
    "edit-find": "search",
    "download": "arrow-down",
    "downloading": "arrow-down",
    "stalledDL": "arrow-down",
    "upload": "arrow-up",
    "stalledUP": "arrow-up",
    "checked-completed": "check",
    "filter-all": "grid",
    "filter-active": "zap",
    "filter-stalled": "clock",
    "queued": "clock",
    "error": "alert",
    "dialog-warning": "alert",
    "tracker-error": "alert",
    "tracker-warning": "alert",
    "tags": "tag",
    "trackers": "server",
    "network-server": "server",
    "trackerless": "globe",
    "directory": "folder",
    "folder-documents": "folder",
    "folder-remote": "folder",
    "set-location": "folder",
    "force-recheck": "refresh",
    "reannounce": "refresh",
    "view-refresh": "refresh",
    "speedometer": "gauge",
    "slow": "gauge",
    "slow_off": "gauge",
    "view-statistics": "bar-chart",
    "chart-line": "bar-chart",
    "edit-copy": "copy",
    "edit-rename": "pencil",
    "application-rss": "rss",
    "loading": "spinner",
    "network-connect": "plug",
    "connected": "plug",
    "disconnected": "plug",
    "view-categories": "list",
    "fileicon": "file",
    "torrent-creator": "file",
}

# Stroke colours: muted ink in light mode, light ink in dark mode; status
# icons take their status colour.
STATUS = {
    "downloading": "accent",
    "download": "accent",
    "stalledDL": "warn",
    "upload": "ok",
    "stalledUP": "ok",
    "checked-completed": "ok",
    "error": "bad",
    "dialog-warning": "warn",
    "tracker-error": "bad",
    "tracker-warning": "warn",
    "filter-active": "accent",
    "slow": "warn",
}


def svg(d: str, colour: str) -> str:
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" '
        f'fill="none" stroke="{colour}" stroke-width="2" stroke-linecap="round" '
        f'stroke-linejoin="round"><path d="{d}"/></svg>'
    )


def recolour(text: str, colour: str) -> str:
    text = re.sub(r'stroke="[^"]*"', f'stroke="{colour}"', text)
    return re.sub(r'stroke-width="[^"]*"', 'stroke-width="2"', text)


def main() -> None:
    if OUT.exists():
        shutil.rmtree(OUT)
    (OUT / "icons" / "dark").mkdir(parents=True)
    (OUT / "fonts").mkdir()
    config = {"colors": palette(0), "colors.dark": palette(1)}
    (OUT / "config.json").write_text(json.dumps(config, indent=2) + "\n", encoding="utf8")
    (OUT / "stylesheet.qss").write_text(STYLESHEET, encoding="utf8")
    for mode, folder in ((0, OUT / "icons"), (1, OUT / "icons" / "dark")):
        tones = {k: oklch(*v[mode]) for k, v in TOKENS.items()}
        for qid, d in EXTRA.items():
            colour = tones[STATUS.get(qid, "ink2")]
            (folder / f"{qid}.svg").write_text(svg(d, colour), encoding="utf8")
        for qid, name in FROM_ASSETS.items():
            colour = tones[STATUS.get(qid, "ink2")]
            src = (REPO / "assets" / "icons" / f"{name}.svg").read_text(encoding="utf8")
            (folder / f"{qid}.svg").write_text(recolour(src, colour), encoding="utf8")
    for f in (REPO / "assets" / "fonts").glob("InstrumentSans-*.ttf"):
        shutil.copy(f, OUT / "fonts" / f.name)
    shutil.copy(REPO / "assets" / "fonts" / "OFL-InstrumentSans.txt", OUT / "fonts")
    shutil.copy(REPO / "assets" / "icons" / "LICENSE.md", OUT / "icons" / "LICENSE.md")
    count = len(EXTRA) + len(FROM_ASSETS)
    print(f"wrote {OUT}: palette, stylesheet, {count} icons x 2 modes, fonts")


if __name__ == "__main__":
    main()
