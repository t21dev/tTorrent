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

namespace DoseDev
{
    // Developer screenshots, rendered off-screen so nothing on the desktop is
    // touched or captured. When TTORRENT_UISHOT_DIR is set, a few seconds
    // after start the app saves main.png, one menu-<name>.png per menu and
    // options.png into that folder, then quits. Does nothing otherwise.
    void scheduleShots(QMainWindow *window);
}
