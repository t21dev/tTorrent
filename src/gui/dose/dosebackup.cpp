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

#include "dosebackup.h"

#include <QCheckBox>
#include <QApplication>
#include <QPointer>
#include <QCoreApplication>
#include <QDateTime>
#include <QDesktopServices>
#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QFileDialog>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QMainWindow>
#include <QMessageBox>
#include <QPushButton>
#include <QStandardPaths>
#include <QTimer>
#include <QUrl>
#include <QVBoxLayout>

#include "base/bittorrent/session.h"
#include "base/global.h"
#include "base/logger.h"
#include "base/path.h"
#include "base/profile.h"
#include "base/settingsstorage.h"
#include "base/ttorrentversion.h"
#include "doseimport.h"

namespace
{
    const QString KEY_DAILY = u"tTorrent/BackupDaily"_s;
    const QString KEY_LAST = u"tTorrent/BackupLast"_s;
    const QString MARKER = u"restore-backup.pending"_s;
    const QString PREFIX = u"tTorrent backup "_s;
    const int KEEP_DAILY = 3;

    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseBackup", text);
    }

    QString configDir()
    {
        return Profile::instance()->location(SpecialFolder::Config).data();
    }

    QString dataDir()
    {
        return Profile::instance()->location(SpecialFolder::Data).data();
    }

    // Documents	Torrent backups; a custom profile (--profile) keeps them inside itself.
    QString backupsDir()
    {
        const Path root = Profile::instance()->rootPath();
        if (!root.isEmpty())
            return (root / Path(u"backups"_s)).data();
        return QStandardPaths::writableLocation(QStandardPaths::DocumentsLocation) + u"/tTorrent backups"_s;
    }

    // Copies a folder's files into another, replacing what is there.
    int copyTree(const QString &from, const QString &to, const QStringList &skip = {})
    {
        int copied = 0;
        if (!QDir(from).exists())
            return 0;
        QDirIterator it {from, QDir::Files, QDirIterator::Subdirectories};
        while (it.hasNext())
        {
            const QString source = it.next();
            const QString relative = source.mid(from.size() + 1);
            if (std::any_of(skip.cbegin(), skip.cend(), [&relative](const QString &s) { return relative.startsWith(s) || relative.endsWith(s); }))
                continue;
            const QString target = to + u'/' + relative;
            QDir().mkpath(QFileInfo(target).absolutePath());
            QFile::remove(target);
            if (QFile::copy(source, target))
                ++copied;
        }
        return copied;
    }

    bool isBackup(const QString &folder)
    {
        return QFile::exists(folder + u"/config/tTorrent.ini"_s);
    }

    // Daily backups beyond the newest three go; manual ones are never removed.
    void pruneDaily()
    {
        QDir dir {backupsDir()};
        QStringList daily = dir.entryList({PREFIX + u"*(daily)"_s}, QDir::Dirs, QDir::Name);
        while (daily.size() > KEEP_DAILY)
            QDir(dir.filePath(daily.takeFirst())).removeRecursively();
    }
}

QString DoseBackup::backupNow(const QString &label)
{
    SettingsStorage::instance()->save();
    const QString name = PREFIX + QDateTime::currentDateTime().toString(u"yyyy-MM-dd HHmm"_s) + (label.isEmpty() ? QString() : (u" ("_s + label + u')'));
    QString folder = backupsDir() + u'/' + name;
    for (int n = 2; QDir(folder).exists(); ++n)
        folder = backupsDir() + u'/' + name + u" " + QString::number(n);
    if (!QDir().mkpath(folder))
        return {};
    copyTree(configDir(), folder + u"/config"_s, {u".pending"_s, u".lock"_s});
    const int files = copyTree(dataDir() + u"/BT_backup"_s, folder + u"/data/BT_backup"_s);
    copyTree(dataDir() + u"/rss"_s, folder + u"/data/rss"_s);
    const int torrents = int(QDir(folder + u"/data/BT_backup"_s).entryList({u"*.fastresume"_s}, QDir::Files).size());
    QFile about {folder + u"/backup.json"_s};
    if (about.open(QIODevice::WriteOnly))
    {
        about.write(QJsonDocument(QJsonObject {{u"app"_s, u"tTorrent"_s}, {u"version"_s, QStringLiteral(TTORRENT_VERSION)}
            , {u"created"_s, QDateTime::currentDateTime().toString(Qt::ISODate)}, {u"torrents"_s, torrents}}).toJson());
    }
    if (!isBackup(folder))
    {
        QDir(folder).removeRecursively();
        return {};
    }
    SettingsStorage::instance()->storeValue(KEY_LAST, QDateTime::currentDateTimeUtc());
    LogMsg(tr("Backed up settings and %1 torrents (%2 files) to \"%3\".").arg(QString::number(torrents), QString::number(files), QDir::toNativeSeparators(folder)));
    return folder;
}

void DoseBackup::backupSoon(QObject *context, const QString &label, const std::function<void (const QString &)> &done)
{
    if (BitTorrent::Session *session = BitTorrent::Session::instance())
        session->saveChangedResumeData();
    // Saving runs in the background; three seconds covers it with room to spare.
    QTimer::singleShot(3000, context, [label, done] { done(backupNow(label)); });
}

void DoseBackup::askAndRestore(QWidget *parent)
{
    const QString folder = QFileDialog::getExistingDirectory(parent, tr("Choose a tTorrent backup folder"), backupsDir());
    if (folder.isEmpty())
        return;
    if (!isBackup(folder))
    {
        QMessageBox::warning(parent, tr("Restore"), tr("That folder isn't a tTorrent backup (it has no config\\tTorrent.ini)."));
        return;
    }
    const int torrents = int(QDir(folder + u"/data/BT_backup"_s).entryList({u"*.fastresume"_s}, QDir::Files).size());
    const QString text = tr("Restore settings and %1 torrents from:\n%2\n\nYour current settings and torrent list are replaced. "
        "A backup of them is made first. Downloaded files aren't touched. tTorrent restarts to finish.")
        .arg(QString::number(torrents), QDir::toNativeSeparators(folder));
    if (QMessageBox::question(parent, tr("Restore"), text, (QMessageBox::Ok | QMessageBox::Cancel), QMessageBox::Ok) != QMessageBox::Ok)
        return;
    QPointer<QWidget> guard {parent};
    backupSoon(parent, tr("before restore"), [guard, folder](const QString &safety)
    {
        if (safety.isEmpty())
        {
            QMessageBox::warning(guard, tr("Restore"), tr("Couldn't make a backup of the current state first, so nothing was restored."));
            return;
        }
        QFile marker {configDir() + u'/' + MARKER};
        if (!marker.open(QIODevice::WriteOnly))
            return;
        marker.write(folder.toUtf8());
        marker.close();
        DoseImport::restart();
    });
}

void DoseBackup::runPendingRestore()
{
    QFile marker {configDir() + u'/' + MARKER};
    if (!marker.exists() || !marker.open(QIODevice::ReadOnly))
        return;
    const QString folder = QString::fromUtf8(marker.readAll()).trimmed();
    marker.close();
    marker.remove();
    if (!isBackup(folder))
        return;
    // The torrent list is replaced, not merged, so removed torrents stay removed.
    QDir(dataDir() + u"/BT_backup"_s).removeRecursively();
    const int files = copyTree(folder + u"/data/BT_backup"_s, dataDir() + u"/BT_backup"_s);
    copyTree(folder + u"/data/rss"_s, dataDir() + u"/rss"_s);
    copyTree(folder + u"/config"_s, configDir());
    LogMsg(tr("Restored settings and the torrent list (%1 files) from \"%2\".").arg(QString::number(files), QDir::toNativeSeparators(folder)));
}

void DoseBackup::install(QMainWindow *window)
{
    // Daily, five minutes after start, when it's been 20 hours since the last one.
    QTimer::singleShot(5 * 60 * 1000, window, []
    {
        if (!SettingsStorage::instance()->loadValue<bool>(KEY_DAILY, false) || qEnvironmentVariableIsSet("TTORRENT_UISHOT_DIR"))
            return;
        const QDateTime last = SettingsStorage::instance()->loadValue<QDateTime>(KEY_LAST);
        if (last.isValid() && (last.secsTo(QDateTime::currentDateTimeUtc()) < (20 * 60 * 60)))
            return;
        backupSoon(qApp, tr("daily"), [](const QString &folder)
        {
            if (!folder.isEmpty())
                pruneDaily();
        });
    });
}

QWidget *DoseBackup::settingsGroup()
{
    auto *group = new QGroupBox(tr("Backup"));
    group->setObjectName(u"doseBackup"_s);
    auto *column = new QVBoxLayout(group);
    auto *note = new QLabel(tr("Settings, categories, tags, RSS and the torrent list with each torrent's progress. Downloaded files aren't included."));
    note->setWordWrap(true);
    note->setObjectName(u"doseMuted"_s);
    column->addWidget(note);
    auto *row = new QHBoxLayout;
    auto *now = new QPushButton(tr("Back up now"));
    auto *restore = new QPushButton(tr("Restore from a backup..."));
    auto *open = new QPushButton(tr("Open backups folder"));
    for (QPushButton *b : {now, restore, open})
    {
        b->setAutoDefault(false);
        row->addWidget(b);
    }
    row->addStretch();
    column->addLayout(row);
    auto *daily = new QCheckBox(tr("Back up every day (keeps the last %1)").arg(KEEP_DAILY));
    daily->setObjectName(u"doseBackupDaily"_s);
    daily->setChecked(SettingsStorage::instance()->loadValue<bool>(KEY_DAILY, false));
    column->addWidget(daily);
    auto *status = new QLabel;
    status->setObjectName(u"doseMuted"_s);
    status->setWordWrap(true);
    const QDateTime last = SettingsStorage::instance()->loadValue<QDateTime>(KEY_LAST);
    status->setText(last.isValid() ? tr("Last backup: %1").arg(QLocale().toString(last.toLocalTime(), QLocale::ShortFormat)) : tr("No backup yet."));
    column->addWidget(status);

    QObject::connect(now, &QPushButton::clicked, group, [now, status]
    {
        now->setEnabled(false);
        status->setText(tr("Saving..."));
        backupSoon(status, {}, [now, status](const QString &folder)
        {
            now->setEnabled(true);
            status->setText(folder.isEmpty() ? tr("Couldn't write the backup.") : tr("Saved to %1").arg(QDir::toNativeSeparators(folder)));
        });
    });
    QObject::connect(restore, &QPushButton::clicked, group, [group] { askAndRestore(group->window()); });
    QObject::connect(open, &QPushButton::clicked, group, []
    {
        QDir().mkpath(backupsDir());
        QDesktopServices::openUrl(QUrl::fromLocalFile(backupsDir()));
    });
    return group;
}

void DoseBackup::saveSettings(QWidget *group)
{
    if (auto *daily = group ? group->findChild<QCheckBox *>(u"doseBackupDaily"_s) : nullptr)
        SettingsStorage::instance()->storeValue(KEY_DAILY, daily->isChecked());
}
