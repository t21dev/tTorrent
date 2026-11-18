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

#include <QIcon>

namespace BitTorrent
{
    class Torrent;
}

// An icon for what a torrent holds, shown before its name in the transfer
// list: video, audio, image, archive, disc image, app, document, folder or
// file (a magnet until the metadata arrives). Settings › Look › Torrent icons
// turns it off (Appearance/TypeIcons).
namespace DoseTypes
{
    bool enabled();
    void setEnabled(bool on);
    QIcon iconFor(const BitTorrent::Torrent *torrent);
    // Forget cached kinds (after the theme changes the icons' colours).
    void clearCache();
}
