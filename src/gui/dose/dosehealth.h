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

class QDialog;

namespace BitTorrent
{
    class TorrentDescriptor;
}

// Health in the Add dialog: before adding, asks up to five of the torrent's
// trackers how many seeds and peers it has (HTTP scrape, BEP 48; UDP scrape,
// BEP 15) and shows the best answer beside Size and Date.
namespace DoseHealth
{
    void check(QDialog *dialog, const BitTorrent::TorrentDescriptor &torrent);
}
