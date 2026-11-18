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
class QListWidget;
class QSplitter;
class QStackedWidget;

// The Dose layouts for qBittorrent's dialogs, applied after their generated
// setupUi() so the upstream .ui files stay untouched and merges stay easy.
// Every step finds its widgets by object name and skips itself if one is
// missing, so an upstream rename degrades to the stock layout, not a crash.
namespace DoseDialogs
{
    // Settings: a sidebar with a "Settings" heading, a search field and the
    // section list; a heading with the current section's name over the page.
    // Search hides sections with nothing matching and opens the first match.
    void decorateOptions(QDialog *dialog, QSplitter *splitter, QListWidget *sections, QStackedWidget *pages);

    // Add torrent: a header with the torrent's name and its size and date; the
    // options on the left, the files on the right with "Torrent information"
    // folded under them; "Never show again" and the buttons on one footer row.
    void decorateAddTorrent(QDialog *dialog);
}
