"""Installs a Qt 6 desktop build for Windows (MSVC 2022, x64) from download.qt.io.

    python -I ttorrent/scripts/install-qt.py 6.12.1 L:\\dev\\Qt

Since Qt 6.12 the online repository keeps each target in its own folder
(qt6_6121/qt6_6121_msvc2022_64/), which aqtinstall 3.3.0 can't read yet. This
script reads that folder's Updates.xml, downloads the essentials package
(qtbase, qtsvg, qtdeclarative, qttools, qttranslations; the same set aqtinstall
installs by default), checks each archive's SHA-1 against the repository's .sha1 file, and
unpacks them to <output>/<version>/msvc2022_64.

download.qt.io sends each download to a nearby mirror, and some are very slow
(1 KB/s seen on 2026-11-23). Set TTORRENT_QT_MIRROR to a mirror's base URL
(e.g. https://ftp.fau.de/qtproject) to fetch the archives from there. The
package list and the SHA-1 files still come from download.qt.io, so a mirror
can't change what gets installed.
"""
import hashlib
import os
import sys
import urllib.request
import xml.etree.ElementTree as ET

import py7zr

REPO = "online/qtsdkrepository/windows_x86/desktop"
BASE = f"https://download.qt.io/{REPO}"
MIRROR = os.environ.get("TTORRENT_QT_MIRROR", "").rstrip("/")
SKIP = ("qtdoc-", "opengl32sw-")  # documentation and the software OpenGL fallback aren't needed


def fetch(url):
    with urllib.request.urlopen(url, timeout=120) as r:
        return r.read()


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    version, output = sys.argv[1], sys.argv[2]
    tag = "qt6_" + version.replace(".", "")
    folder = f"{BASE}/{tag}/{tag}_msvc2022_64"
    archive_folder = f"{MIRROR}/{REPO}/{tag}/{tag}_msvc2022_64" if MIRROR else folder
    updates = ET.fromstring(fetch(f"{folder}/Updates.xml"))
    package = next(p for p in updates.findall("PackageUpdate") if p.findtext("Name") == f"qt.qt6.{version.replace('.', '')}.win64_msvc2022_64")
    name, package_version = package.findtext("Name"), package.findtext("Version")
    archives = [a.strip() for a in package.findtext("DownloadableArchives").split(",") if a.strip() and not a.strip().startswith(SKIP)]
    root = os.path.join(output, version, "msvc2022_64")
    os.makedirs(root, exist_ok=True)
    downloads = os.path.join(output, "_downloads", version)
    os.makedirs(downloads, exist_ok=True)
    for archive in archives:
        url = f"{folder}/{name}/{package_version}{archive}"
        archive_url = f"{archive_folder}/{name}/{package_version}{archive}"
        path = os.path.join(downloads, archive)
        if not os.path.exists(path):
            print("download", archive, flush=True)
            data = fetch(archive_url)
            expected = fetch(url + ".sha1").decode().split()[0]
            if hashlib.sha1(data).hexdigest() != expected:
                sys.exit(f"checksum mismatch: {archive}")
            with open(path, "wb") as f:
                f.write(data)
        print("unpack", archive, flush=True)
        # The 6.12 archives hold bin/, lib/... with no version folder in front.
        with py7zr.SevenZipFile(path) as z:
            z.extractall(root)
    # Like aqtinstall: a qt.conf so qmake and windeployqt find the install where it is.
    with open(os.path.join(root, "bin", "qt.conf"), "w") as f:
        f.write("[Paths]\nPrefix=..\n")
    print("installed", root)


if __name__ == "__main__":
    main()
