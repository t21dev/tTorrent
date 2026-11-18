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

#include <functional>

#include <QString>

class QMainWindow;
class QObject;
class QWidget;

// Back up and restore (qBittorrent issue #6236): settings, categories, tags,
// RSS, watched folders and the torrent list with each torrent's progress,
// copied into a dated folder (Documents\tTorrent backups by default).
// Restoring replaces them at the next start, after keeping a copy of the
// current ones. An optional daily backup keeps the last three.
namespace DoseBackup
{
    void install(QMainWindow *window);

    // Writes a backup; returns its folder, or an empty string on failure.
    // Torrents changed in the last seconds may not be saved yet: use backupSoon.
    QString backupNow(const QString &label = {});
    // Asks every changed torrent to save its state, waits for it, then backs
    // up and calls `done` with the folder (empty on failure).
    void backupSoon(QObject *context, const QString &label, const std::function<void (const QString &)> &done);
    // Asks for a backup folder, keeps a copy of the current state and restarts to restore.
    void askAndRestore(QWidget *parent);
    // At start, before settings load (from DoseImport::runPending).
    void runPendingRestore();

    // Settings › Behavior › "Backup", and saving it.
    QWidget *settingsGroup();
    void saveSettings(QWidget *group);
}
