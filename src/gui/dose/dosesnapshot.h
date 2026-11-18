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

class QAbstractItemView;

// The list at start: when tTorrent quits it keeps a picture of the transfer
// list's visible rows (in the profile's cache folder); at the next start that
// picture shows at once and cross-fades to the live list as soon as the
// torrents have loaded, so the window never opens on an empty list.
namespace DoseSnapshot
{
    void attach(QAbstractItemView *view);
}
