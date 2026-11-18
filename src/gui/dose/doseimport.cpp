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

#include "doseimport.h"

#include <QAction>
#include <QApplication>
#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QMainWindow>
#include <QMessageBox>
#include <QProcess>
#include <QSettings>
#include <QThread>

#ifdef Q_OS_WIN
#include <windows.h>
#else
#include <csignal>
#endif

#include "base/global.h"
#include "base/logger.h"
#include "base/path.h"
#include "base/profile.h"
#include "dosebackup.h"

namespace
{
    const QString MARKER = u"import-qbittorrent.pending"_s;

    QString ourConfigDir()
    {
        return Profile::instance()->location(SpecialFolder::Config).data();
    }

    QString ourDataDir()
    {
        return Profile::instance()->location(SpecialFolder::Data).data();
    }

    // qBittorrent's default folders on this system.
    struct Folders
    {
        QString config;
        QString data;
    };

    Folders qbittorrentFolders()
    {
#if defined(Q_OS_WIN)
        return {qEnvironmentVariable("APPDATA") + u"/qBittorrent"_s, qEnvironmentVariable("LOCALAPPDATA") + u"/qBittorrent"_s};
#elif defined(Q_OS_MACOS)
        return {QDir::homePath() + u"/Library/Preferences/qBittorrent"_s, QDir::homePath() + u"/Library/Application Support/qBittorrent"_s};
#else
        return {QDir::homePath() + u"/.config/qBittorrent"_s, QDir::homePath() + u"/.local/share/qBittorrent"_s};
#endif
    }

    // Copies a folder's files into another, keeping files already there.
    int copyTree(const QString &from, const QString &to)
    {
        int copied = 0;
        if (!QDir(from).exists())
            return 0;
        QDirIterator it {from, QDir::Files, QDirIterator::Subdirectories};
        while (it.hasNext())
        {
            const QString source = it.next();
            const QString target = to + source.mid(from.size());
            QDir().mkpath(QFileInfo(target).absolutePath());
            if (!QFile::exists(target) && QFile::copy(source, target))
                ++copied;
        }
        return copied;
    }

    void copyFile(const QString &from, const QString &to)
    {
        if (QFile::exists(from))
        {
            QFile::remove(to);
            QFile::copy(from, to);
        }
    }
}

int DoseImport::qbittorrentTorrentCount()
{
    const QDir backup {qbittorrentFolders().data + u"/BT_backup"_s};
    if (!backup.exists())
        return -1;
    return int(backup.entryList({u"*.fastresume"_s}, QDir::Files).size());
}

void DoseImport::askAndRestart(QWidget *parent)
{
    const int count = qbittorrentTorrentCount();
    if (count < 0)
    {
        QMessageBox::information(parent, QApplication::translate("DoseImport", "Import from qBittorrent")
            , QApplication::translate("DoseImport", "No qBittorrent profile was found on this computer."));
        return;
    }
    const QString text = QApplication::translate("DoseImport",
        "Found %1 torrents in qBittorrent.\n\n"
        "tTorrent will bring over the torrents with their progress (nothing is rechecked), categories, tags, "
        "RSS feeds and rules, watched folders and settings. Your tTorrent look stays as it is. "
        "qBittorrent's own files are only read.\n\n"
        "Close qBittorrent first. tTorrent restarts to finish the import.").arg(count);
    if (QMessageBox::question(parent, QApplication::translate("DoseImport", "Import from qBittorrent"), text
            , (QMessageBox::Yes | QMessageBox::Cancel), QMessageBox::Yes) != QMessageBox::Yes)
    {
        return;
    }

    QFile marker {ourConfigDir() + u'/' + MARKER};
    if (!marker.open(QIODevice::WriteOnly))
        return;
    marker.close();
    restart();
}

void DoseImport::restart()
{
    // Start the next copy, told to wait for this one, then exit cleanly.
    QProcess next;
    next.setProgram(QCoreApplication::applicationFilePath());
    next.setArguments(QCoreApplication::arguments().mid(1));
    QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    env.insert(u"TTORRENT_WAIT_PID"_s, QString::number(QCoreApplication::applicationPid()));
    next.setProcessEnvironment(env);
    next.startDetached();

    for (QWidget *top : QApplication::topLevelWidgets())
    {
        if (auto *main = qobject_cast<QMainWindow *>(top))
        {
            if (auto *exit = main->findChild<QAction *>(u"actionExit"_s))
            {
                exit->trigger();
                return;
            }
        }
    }
    QApplication::quit();
}

void DoseImport::runPending()
{
    DoseBackup::runPendingRestore();
    const QString marker = ourConfigDir() + u'/' + MARKER;
    if (!QFile::exists(marker))
        return;
    QFile::remove(marker);

    const Folders qb = qbittorrentFolders();
    const QString config = ourConfigDir();
    const QString data = ourDataDir();
    QDir().mkpath(config);
    QDir().mkpath(data);

    const int torrents = copyTree(qb.data + u"/BT_backup"_s, data + u"/BT_backup"_s) / 2;
    copyTree(qb.data + u"/rss"_s, data + u"/rss"_s);
    copyTree(qb.config + u"/rss"_s, config + u"/rss"_s);
    for (const QString &name : {u"categories.json"_s, u"watched_folders.json"_s})
        copyFile(qb.config + u'/' + name, config + u'/' + name);
    copyFile(qb.config + u"/qBittorrent-data.ini"_s, config + u"/tTorrent-data.ini"_s);

    // Settings: qBittorrent's values, except tTorrent's own look.
    const QSettings theirs {qb.config + u"/qBittorrent.ini"_s, QSettings::IniFormat};
    QSettings ours {config + u"/tTorrent.ini"_s, QSettings::IniFormat};
    for (const QString &key : theirs.allKeys())
    {
        if (key.startsWith(u"Appearance/") || key.startsWith(u"LegalNotice/") || key.startsWith(u"Meta/"))
            continue;
        // A custom qBittorrent theme would replace tTorrent's look, and the log
        // folder points into qBittorrent's own profile (tTorrent must never write there).
        if ((key == u"Preferences/General/UseCustomUITheme") || (key == u"Preferences/General/CustomUIThemePath")
            || (key == u"Application/FileLogger/Path"))
        {
            continue;
        }
        ours.setValue(key, theirs.value(key));
    }
    ours.sync();

    LogMsg(QApplication::translate("DoseImport", "Imported %1 torrents and settings from qBittorrent.").arg(torrents));
}

void DoseImport::waitForPreviousInstance()
{
    const qint64 pid = qEnvironmentVariableIntValue("TTORRENT_WAIT_PID");
    qunsetenv("TTORRENT_WAIT_PID");
    if (pid <= 0)
        return;
#ifdef Q_OS_WIN
    if (HANDLE process = ::OpenProcess(SYNCHRONIZE, FALSE, static_cast<DWORD>(pid)))
    {
        ::WaitForSingleObject(process, 60000);
        ::CloseHandle(process);
    }
#else
    for (int i = 0; (i < 600) && (::kill(static_cast<pid_t>(pid), 0) == 0); ++i)
        QThread::msleep(100);
#endif
}
