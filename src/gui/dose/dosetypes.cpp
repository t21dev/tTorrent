/*
 * Bittorrent Client using Qt and libtorrent.
 * Copyright (C) 2026  t21 dev
 *
 * This program is free software; you can redistribute it and/or
 * modify it under the terms of the GNU General Public License
 * as published by the Free Software Foundation; either version 2
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 */

#include "dosetypes.h"

#include <QHash>
#include <QString>

#include "base/bittorrent/infohash.h"
#include "base/bittorrent/torrent.h"
#include "base/global.h"
#include "base/path.h"
#include "base/settingsstorage.h"
#include "dosepaint.h"
#include "gui/uithememanager.h"

namespace
{
    const QString KEY = u"Appearance/TypeIcons"_s;

    int &enabledCache()
    {
        static int cached = -1; // -1: not read yet
        return cached;
    }

    // The kind of one file, from its extension.
    QString kindOf(const QString &fileName)
    {
        static const QHash<QString, QString> kinds = []
        {
            QHash<QString, QString> map;
            const auto add = [&map](const QString &kind, std::initializer_list<const char *> exts)
            {
                for (const char *e : exts)
                    map.insert(QString::fromLatin1(e), kind);
            };
            add(u"video"_s, {"mkv", "mp4", "avi", "mov", "wmv", "m4v", "webm", "ts", "m2ts", "flv", "mpg", "mpeg", "vob", "3gp"});
            add(u"audio"_s, {"mp3", "flac", "m4a", "aac", "ogg", "opus", "wav", "wma", "ape", "alac", "m4b"});
            add(u"image"_s, {"jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "heic", "psd"});
            add(u"archive"_s, {"zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz", "zst", "cab"});
            add(u"disc"_s, {"iso", "img", "bin", "cue", "nrg", "dmg", "vhd", "vhdx", "mdf"});
            add(u"app"_s, {"exe", "msi", "apk", "appimage", "deb", "rpm", "pkg", "msix", "appx"});
            add(u"document"_s, {"pdf", "epub", "mobi", "azw3", "doc", "docx", "txt", "cbr", "cbz", "djvu", "odt"});
            return map;
        }();
        const qsizetype dot = fileName.lastIndexOf(u'.');
        if (dot < 0)
            return u"file"_s;
        // Split archives (.r00, .part1.rar) count as archives.
        const QString ext = fileName.mid(dot + 1).toLower();
        if ((ext.size() == 3) && (ext.at(0) == u'r') && ext.at(1).isDigit() && ext.at(2).isDigit())
            return u"archive"_s;
        return kinds.value(ext, u"file"_s);
    }

    // One file: its kind. Several: the largest file's kind when it is at
    // least half the torrent (a film with its subtitles), else a folder.
    QString kindOf(const BitTorrent::Torrent *torrent)
    {
        if (!torrent->hasMetadata())
            return u"magnet"_s;
        const int count = torrent->filesCount();
        if (count <= 0)
            return u"file"_s;
        if (count == 1)
            return kindOf(torrent->filePath(0).filename());
        int largest = 0;
        qlonglong total = 0;
        for (int i = 0; i < count; ++i)
        {
            const qlonglong size = torrent->fileSize(i);
            total += size;
            if (size > torrent->fileSize(largest))
                largest = i;
        }
        const QString kind = kindOf(torrent->filePath(largest).filename());
        return ((kind != u"file") && (torrent->fileSize(largest) * 2 >= total)) ? kind : u"folder"_s;
    }

    QHash<BitTorrent::TorrentID, QString> &cache()
    {
        static QHash<BitTorrent::TorrentID, QString> kinds;
        return kinds;
    }

    QIcon iconForKind(const QString &kind)
    {
        // Icons are tinted per theme: start over when the colours change.
        static QHash<QString, QIcon> icons;
        static QString colours;
        const QString now = DosePaint::token(u"muted"_s).name() + DosePaint::token(u"accent"_s).name();
        if (now != colours)
        {
            icons.clear();
            colours = now;
        }
        if (!icons.contains(kind))
        {
            const QString id = (kind == u"folder") ? u"directory"_s
                : (kind == u"file") ? u"fileicon"_s
                : (kind == u"magnet") ? u"torrent-magnet"_s
                : (u"dose-type-"_s + kind);
            icons.insert(kind, UIThemeManager::instance()->getIcon(id));
        }
        return icons.value(kind);
    }
}

bool DoseTypes::enabled()
{
    int &cached = enabledCache();
    if (cached < 0)
        cached = SettingsStorage::instance()->loadValue<bool>(KEY, true) ? 1 : 0;
    return DosePaint::active() && (cached == 1);
}

void DoseTypes::setEnabled(const bool on)
{
    enabledCache() = on ? 1 : 0;
    SettingsStorage::instance()->storeValue(KEY, on);
}

QIcon DoseTypes::iconFor(const BitTorrent::Torrent *torrent)
{
    if (!torrent)
        return {};
    auto &kinds = cache();
    const BitTorrent::TorrentID id = torrent->id();
    auto it = kinds.constFind(id);
    if (it == kinds.cend())
    {
        const QString kind = kindOf(torrent);
        // A magnet's kind changes once its metadata arrives, so it isn't kept.
        if (kind == u"magnet")
            return iconForKind(kind);
        it = kinds.insert(id, kind);
    }
    return iconForKind(it.value());
}

void DoseTypes::clearCache()
{
    cache().clear();
}
