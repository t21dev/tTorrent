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

// tTorrent's version, shown to people (About, log, --version, crash reports).
// QBT_VERSION stays qBittorrent's: libtorrent's peer ID and tracker user
// agent are built from it, and private trackers allow clients by it.
#define TTORRENT_VERSION "0.6.2"
#define TTORRENT_VERSION_LABEL "stable"
