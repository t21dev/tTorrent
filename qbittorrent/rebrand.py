"""Rebrand the qBittorrent fork as tTorrent. Idempotent: run after merging a
new qBittorrent release. Keeps qBittorrent's copyright (GPL) and says the app
is based on it.

What changes:
- application name and organisation: settings live in %APPDATA%\\tTorrent and
  %LOCALAPPDATA%\\tTorrent, never in qBittorrent's folders
- the settings file name (tTorrent.ini)
- executable ttorrent(.exe), macOS bundle name, Windows file details and icon
- window title, message-box titles
- app and tray icons, through the built-in Dose theme

Run: python qbittorrent/rebrand.py
"""

import re
import shutil
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FORK = Path(r"T:\Work\t21\qbittorrent-fork")
SRC = FORK / "src"


def edit(path: Path, pairs: list[tuple[str, str]]) -> None:
    s = path.read_text(encoding="utf8")
    for old, new in pairs:
        if new in s:
            continue
        if old not in s:
            raise SystemExit(f"{path}: not found: {old[:70]!r}")
        s = s.replace(old, new)
    path.write_text(s, encoding="utf8")


def main() -> None:
    edit(SRC / "app" / "application.cpp", [
        ('setApplicationName(u"qBittorrent"_s);', 'setApplicationName(u"tTorrent"_s);'),
        ('setOrganizationDomain(u"qbittorrent.org"_s);', 'setOrganizationDomain(u"t21.dev"_s);'),
        ('setDesktopFileName(u"org.qbittorrent.qBittorrent"_s);', 'setDesktopFileName(u"dev.t21.tTorrent"_s);'),
    ])
    edit(SRC / "base" / "settingsstorage.cpp", [
        ('m_nativeSettingsName {u"qBittorrent"_s}', 'm_nativeSettingsName {u"tTorrent"_s}'),
    ])
    edit(SRC / "gui" / "mainwindow.cpp", [
        ('m_windowTitle = QStringLiteral("qBittorrent " QBT_VERSION)', 'm_windowTitle = QStringLiteral("tTorrent " QBT_VERSION)'),
    ])
    edit(SRC / "app" / "CMakeLists.txt", [
        ("set_target_properties(qbt_app PROPERTIES OUTPUT_NAME qbittorrent)", "set_target_properties(qbt_app PROPERTIES OUTPUT_NAME ttorrent)"),
        ("set_target_properties(qbt_app PROPERTIES OUTPUT_NAME qbittorrent-nox)", "set_target_properties(qbt_app PROPERTIES OUTPUT_NAME ttorrent-nox)"),
        ('MACOSX_BUNDLE_BUNDLE_NAME "qBittorrent"', 'MACOSX_BUNDLE_BUNDLE_NAME "tTorrent"'),
    ])
    edit(SRC / "qbittorrent.rc", [
        ('"The qBittorrent Project"', '"t21 dev"'),
        ('"qBittorrent - A Bittorrent Client"', '"tTorrent - BitTorrent client, based on qBittorrent"'),
        ('VALUE "InternalName",     "qbittorrent"', 'VALUE "InternalName",     "ttorrent"'),
        ('"Copyright ©2006-2026 The qBittorrent Project"', '"Copyright ©2006-2026 The qBittorrent Project, ©2026 t21 dev"'),
        ('"qbittorrent.exe"', '"ttorrent.exe"'),
        ('VALUE "ProductName",      "qBittorrent"', 'VALUE "ProductName",      "tTorrent"'),
    ])
    # Message-box titles in the window code (not D-Bus or power-management ids).
    for f in (SRC / "gui").rglob("*.cpp"):
        rel = f.relative_to(SRC / "gui").as_posix()
        if rel.startswith(("notifications/", "powermanagement/")):
            continue
        s = f.read_text(encoding="utf8")
        t = re.sub(r'(QMessageBox::\w+\(\s*[^,]+,\s*)u"qBittorrent"_s', r'\1u"tTorrent"_s', s)
        t = t.replace('new QMessageBox {QMessageBox::Information, u"qBittorrent"_s', 'new QMessageBox {QMessageBox::Information, u"tTorrent"_s')
        if t != s:
            f.write_text(t, encoding="utf8")
    # Icons: the executable's, and the app and tray icons in the built-in theme.
    shutil.copy(REPO / "assets" / "brand" / "ttorrent.ico", SRC / "icons" / "qbittorrent.ico")
    theme = SRC / "gui" / "dose" / "theme" / "icons"
    shutil.copy(REPO / "assets" / "brand" / "ttorrent-256.png", theme / "qbittorrent.png")
    shutil.copy(REPO / "assets" / "brand" / "ttorrent-256.png", theme / "qbittorrent-tray.png")
    sprout = (REPO / "assets" / "icons" / "sprout.svg").read_text(encoding="utf8")
    for name, colour in (("qbittorrent-tray-dark", "#1E1F22"), ("qbittorrent-tray-light", "#FFFFFF")):
        (theme / f"{name}.svg").write_text(re.sub(r'stroke="[^"]*"', f'stroke="{colour}"', sprout), encoding="utf8")
    qrc = SRC / "gui" / "dose" / "theme" / "dosetheme.qrc"
    q = qrc.read_text(encoding="utf8")
    for f in ("icons/qbittorrent.png", "icons/qbittorrent-tray.png", "icons/qbittorrent-tray-dark.svg", "icons/qbittorrent-tray-light.svg"):
        if f"<file>{f}</file>" not in q:
            q = q.replace("    </qresource>", f"        <file>{f}</file>\n    </qresource>")
    qrc.write_text(q, encoding="utf8")
    print("rebranded as tTorrent")


if __name__ == "__main__":
    main()
