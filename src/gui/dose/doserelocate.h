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

#pragma once

#include <QList>

class QAction;
class QMenu;
class QWidget;
class TorrentContentWidget;

namespace BitTorrent
{
    class Torrent;
}

// Relocate (qBittorrent issue #439): files were moved or renamed outside
// tTorrent; point the torrent at where they are now and check them, instead
// of downloading again. Nothing is moved or deleted by relocating itself.
// - A whole torrent: choose the folder that now holds its files. Each file is
//   looked for at its usual path inside it, without the torrent's top folder
//   (the folder was renamed), or by name alone (the files were flattened).
// - Files or a folder in the Content tab: choose the file, or the folder that
//   holds them, wherever it is.
namespace DoseRelocate
{
    // Adds "Relocate..." for a single selected torrent with metadata; returns
    // the action (nullptr when it doesn't apply) for the menu shaper.
    QAction *addTorrentAction(QMenu *menu, QWidget *parent, const QList<BitTorrent::Torrent *> &torrents);
    void addToContentMenu(QMenu *menu, TorrentContentWidget *view);

    // Without dialogs: finds the torrent's files in `folder`, points the
    // torrent there and starts a check. Returns how many files were found.
    int relocateTo(BitTorrent::Torrent *torrent, const QString &folder);
}
