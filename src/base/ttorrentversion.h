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

// tTorrent's version, shown to people (About, log, --version, crash reports,
// the exe's file details). QBT_VERSION stays qBittorrent's: libtorrent's peer
// ID and tracker user agent are built from it, and private trackers allow
// clients by it.
// A release changes all four lines below; the build fails if they disagree.
// This file is also read by the resource compiler (qbittorrent.rc), so it
// holds only #defines outside the RC_INVOKED block.
#define TTORRENT_VERSION "0.6.6"
#define TTORRENT_VERSION_MAJOR 0
#define TTORRENT_VERSION_MINOR 6
#define TTORRENT_VERSION_PATCH 6
#define TTORRENT_VERSION_LABEL "stable"

#ifndef RC_INVOKED
#include <string_view>

#define TTORRENT_STR_(x) #x
#define TTORRENT_STR(x) TTORRENT_STR_(x)
static_assert(std::string_view(TTORRENT_VERSION)
        == std::string_view(TTORRENT_STR(TTORRENT_VERSION_MAJOR) "." TTORRENT_STR(TTORRENT_VERSION_MINOR) "." TTORRENT_STR(TTORRENT_VERSION_PATCH))
    , "TTORRENT_VERSION must match TTORRENT_VERSION_MAJOR.MINOR.PATCH");
#endif
