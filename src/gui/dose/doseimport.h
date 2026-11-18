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

class QWidget;

// Import from qBittorrent (ttorrent/ROADMAP.md): torrents with their progress
// (no recheck), categories, tags, RSS, watched folders and settings. The copy
// runs at the next start, before the torrent engine loads, so nothing is
// overwritten when the running session saves. qBittorrent's files are only
// read. tTorrent's own look settings (Appearance) are kept.
namespace DoseImport
{
    // How many torrents qBittorrent's profile holds; -1 when none is found.
    int qbittorrentTorrentCount();

    // Asks, marks the import as pending and restarts tTorrent.
    void askAndRestart(QWidget *parent);

    // At start, after the profile is known and before settings load: does a
    // pending import. Safe to call every start; does nothing otherwise.
    void runPending();

    // At the very start of a restart: waits for the previous copy to exit
    // (TTORRENT_WAIT_PID), so the new one doesn't hand off to it and quit.
    void waitForPreviousInstance();
}
