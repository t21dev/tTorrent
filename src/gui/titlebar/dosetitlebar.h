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
class QMenuBar;

namespace DoseTitleBar
{
    // Replaces the window's frame with a custom title bar: app icon, the
    // menu bar inline, the window title and minimize/maximize/close, drawn
    // with the palette. Native behaviour stays (moving, resizing, Windows 11
    // snap layouts, the system menu, shadows) through QWindowKit. On macOS
    // the native traffic-light buttons and the system menu bar stay.
    void install(QMainWindow *window, QMenuBar *menuBar);

    // The same frame for a dialog: its title on the left and a Close
    // button, placed above the dialog's own layout. Does nothing on macOS
    // or for a dialog without a layout.
    void installDialog(QDialog *dialog);
}
