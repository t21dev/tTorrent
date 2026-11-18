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

class QAction;
class QLineEdit;
class QMainWindow;

// Keyboard shortcuts: a sheet that lists every command by menu and lets the
// user change its keys (saved in Appearance/Shortcuts), and a command
// palette (Ctrl+K) that runs any menu command by typing part of its name.
namespace DoseShortcuts
{
    // Applies saved custom keys and adds "Keyboard shortcuts" (Ctrl+/) and
    // "Command palette" (Ctrl+K) to the Help menu. Call once the main
    // window's own shortcuts are set.
    void install(QMainWindow *window);

    void showSheet(QMainWindow *window);
    void showPalette(QMainWindow *window);
    // File › Restart tTorrent (also in the tray menu): quits cleanly and
    // starts again. Made once per window.
    QAction *restartAction(QMainWindow *window);
    // One search for the title bar: the filter field keeps filtering the
    // torrent list as you type, and a drop-down under it lists the commands
    // that match (Up/Down to pick, Enter to run). Ctrl+K opens it.
    void attachSearch(QMainWindow *window, QLineEdit *edit);
}
