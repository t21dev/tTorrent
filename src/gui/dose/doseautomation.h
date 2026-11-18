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

#include <QString>

class QMainWindow;
class QWidget;

// tTorrent's own automation (ttorrent/ROADMAP.md › First release):
// - low disk space guard: stops downloads on a drive before it fills and
//   starts them again once there is room;
// - webhooks: a URL called with JSON when a torrent is added, finishes or fails;
// - unpack when finished: .zip, .rar and .7z extracted next to the archive;
// - notifications with actions: a card in the window's corner with
//   "Open folder" and "Show" when a torrent finishes or fails.
// Settings live under "tTorrent/" and are edited in Settings › Downloads.
namespace DoseAutomation
{
    void install(QMainWindow *window);

    // Settings › Downloads › "When something happens": the controls for all of the above.
    QWidget *settingsGroup();
    // Saves the group's controls (called when Settings is applied).
    void saveSettings(QWidget *group);

    bool lowDiskGuard();
    int lowDiskGiB();
    QString webhookUrl();
    bool unpackArchives();
    bool notificationCards();
}
