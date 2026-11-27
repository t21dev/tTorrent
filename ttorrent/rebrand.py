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

BRAND = Path(__file__).resolve().parent / "brand"
FORK = Path(__file__).resolve().parent.parent
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
        ('m_windowTitle = QStringLiteral("qBittorrent " QBT_VERSION)', 'm_windowTitle = u"tTorrent"_s // tTorrent: the version lives in About'),
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
        ('#include "base/version.h"\n', '#include "base/ttorrentversion.h" // tTorrent: the exe\'s details show tTorrent\'s version\n'),
        ('QBT_VERSION_MAJOR,QBT_VERSION_MINOR,QBT_VERSION_BUGFIX,QBT_VERSION_BUILD', 'TTORRENT_VERSION_MAJOR,TTORRENT_VERSION_MINOR,TTORRENT_VERSION_PATCH,0'),
        ('_STR         QBT_VERSION', '_STR         TTORRENT_VERSION'),
        ('_STR      QBT_VERSION', '_STR      TTORRENT_VERSION'),
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
    shutil.copy(BRAND / "ttorrent.ico", SRC / "icons" / "qbittorrent.ico")
    theme = SRC / "gui" / "dose" / "theme" / "icons"
    shutil.copy(BRAND / "ttorrent-256.png", theme / "qbittorrent.png")
    shutil.copy(BRAND / "ttorrent-256.png", theme / "qbittorrent-tray.png")
    # The monochrome tray SVGs are drawn by hand in the theme folder (ours, so
    # a merge never brings qBittorrent's back) and are not written here.
    qrc = SRC / "gui" / "dose" / "theme" / "dosetheme.qrc"
    q = qrc.read_text(encoding="utf8")
    for f in ("icons/qbittorrent.png", "icons/qbittorrent-tray.png", "icons/qbittorrent-tray-dark.svg", "icons/qbittorrent-tray-light.svg"):
        if f"<file>{f}</file>" not in q:
            q = q.replace("    </qresource>", f"        <file>{f}</file>\n    </qresource>")
    qrc.write_text(q, encoding="utf8")
    print("rebranded as tTorrent")


if __name__ == "__main__":
    main()
