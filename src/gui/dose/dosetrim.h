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
class QMainWindow;

// The features the owner left out (ttorrent/ROADMAP.md › Left out): the
// update checker, Donate, the search engine, peer host names, the embedded
// tracker, I2P, email on completion and the alternative Web UI. Their code
// stays (UPSTREAM.md: switch off, never delete); this turns each one off,
// keeps it off, and removes its menu items and settings rows.
namespace DoseTrim
{
    // Main window: menus and actions; forces the features' settings off.
    void applyToMainWindow(QMainWindow *window);
    // Options: hides the rows, groups and pages for those features.
    void applyToOptions(QDialog *dialog);

    // Where Help points instead of qBittorrent's sites.
    const char WEBSITE[] = "https://ttorrent.t21.dev";
    // Help › Documentation: the README on GitHub, kept up to date with each release.
    const char DOCS[] = "https://github.com/t21dev/tTorrent#readme";
    const char ISSUES[] = "https://github.com/t21dev/tTorrent/issues";
}
