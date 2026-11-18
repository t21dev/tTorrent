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

class QMainWindow;
class QWidget;

// tTorrent's own update check, from the GitHub releases of t21dev/tTorrent
// (TTORRENT_UPDATE_REPO overrides the repository, for testing). Checks once a
// day a minute after start, and from Help › Check for updates. A newer
// release shows a card with "What's new" and "Download"; nothing is
// downloaded or installed without the user. The repository must be public:
// GitHub answers 404 to anyone without access to a private one.
namespace DoseUpdate
{
    void install(QMainWindow *window);
    // Checks now and always reports the result (up to date, newer, or failed).
    void checkNow(QMainWindow *window);

    // Settings › Behavior › "Updates", and saving it.
    QWidget *settingsGroup();
    void saveSettings(QWidget *group);
}
