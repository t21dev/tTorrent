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

#include <QHash>
#include <QString>

class QAction;
class QMenu;

namespace DoseTorrentMenu
{
    // Regroups the torrent right-click menu (DESIGN.md › Torrent menu): the
    // everyday actions at the top level, the rest in Files, Download and
    // Trackers submenus, then Queue and Copy, and "Customize this menu" to
    // hide entries (Appearance/TorrentMenuHidden). `named` maps a stable id
    // to each of qBittorrent's actions; only actions it already added to
    // `menu` are placed. Does nothing under a custom UI theme.
    void shape(QMenu *menu, const QHash<QString, QAction *> &named, QMenu *category, QMenu *tags, QMenu *copy);
}
