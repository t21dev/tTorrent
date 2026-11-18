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

#include <QtTypes>

class QLabel;
class QMainWindow;

// Memory that stays flat (ttorrent/ROADMAP.md):
// - samples memory every 30 s for the last 24 hours (about 70 KB);
// - the status bar's Memory label opens a card with the graph since start,
//   the peak and the memory reserved;
// - caps Qt's image cache, and gives freed heap memory back to Windows after
//   the window has been hidden or minimized for a minute;
// - logs how long start-up took: to the window, and until every torrent is ready.
namespace DoseMemory
{
    void install(QMainWindow *window);
    // Makes the status bar's Memory label open the card when clicked.
    void attachToLabel(QLabel *label);

    // Task Manager's "Memory" (private working set) and the memory reserved
    // for the process (private bytes), in bytes; -1 when unknown.
    qint64 privateWorkingSet();
    qint64 privateBytes();
    // Hands free heap pages back to Windows; returns bytes of private memory released.
    qint64 returnFreeMemory();
}
