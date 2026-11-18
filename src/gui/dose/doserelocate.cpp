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

#include "doserelocate.h"

#include <memory>

#include <QAction>
#include <QCoreApplication>
#include <QFileDialog>
#include <QFileInfo>
#include <QMenu>
#include <QMessageBox>
#include <QPointer>

#include "base/bittorrent/session.h"
#include "base/bittorrent/torrent.h"
#include "base/global.h"
#include "base/logger.h"
#include "base/path.h"
#include "gui/torrentcontentwidget.h"
#include "gui/uithememanager.h"

namespace
{
    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseRelocate", text);
    }

    QString title()
    {
        return tr("Relocate");
    }

    // A file appended ".!qB" while incomplete can't be pointed at a finished file by its real name.
    bool blockedByExtension(QWidget *parent)
    {
        if (!BitTorrent::Session::instance()->isAppendExtensionEnabled())
            return false;
        QMessageBox::information(parent, title(), tr("Turn off \"Append .!qB extension to incomplete files\" in Settings › Downloads first, "
            "so tTorrent looks for the files under their real names."));
        return true;
    }

    // Points files at new paths (absolute, or relative to the save path),
    // then checks the whole torrent. Refuses a file whose old copy is still
    // there, so a rename can never move it over the copy the user chose.
    void pointAndCheck(QWidget *parent, BitTorrent::Torrent *torrent, const QHash<int, Path> &targets)
    {
        QStringList refused;
        int pointed = 0;
        for (auto it = targets.cbegin(); it != targets.cend(); ++it)
        {
            if (torrent->filePath(it.key()) == it.value())
                continue;
            const Path current = torrent->actualStorageLocation() / torrent->actualFilePath(it.key());
            const Path wanted = it.value().isAbsolute() ? it.value() : (torrent->savePath() / it.value());
            if (current.exists() && wanted.exists() && (current != wanted))
            {
                refused.append(torrent->filePath(it.key()).toString());
                continue;
            }
            torrent->renameFile(it.key(), it.value());
            ++pointed;
        }
        torrent->forceRecheck();
        LogMsg(tr("Relocated %1 file(s) of \"%2\" and started a check.").arg(QString::number(pointed), torrent->name()));
        if (!refused.isEmpty())
        {
            if (parent)
                QMessageBox::information(parent, title(), tr("These files are still at their old place as well, so they were left as they are:\n\n%1")
                .arg(refused.mid(0, 10).join(u'\n')));
        }
    }

    // Where each file of `torrent` is inside `base`: its usual path, without
    // the top folder (the folder was renamed), or by name (flattened).
    QHash<int, Path> findFiles(const BitTorrent::Torrent *torrent, const Path &base)
    {
        QHash<int, Path> targets;
        for (int i = 0; i < torrent->filesCount(); ++i)
        {
            const Path relative = torrent->filePath(i);
            if (relative.isAbsolute())
                continue;
            QList<Path> candidates {relative};
            const QString text = relative.data();
            if (const qsizetype slash = text.indexOf(u'/'); slash > 0)
                candidates.append(Path(text.mid(slash + 1)));
            candidates.append(Path(relative.filename()));
            for (const Path &candidate : candidates)
            {
                if ((base / candidate).exists())
                {
                    targets.insert(i, candidate);
                    break;
                }
            }
        }
        return targets;
    }

    // Points the torrent at `base` and its files at `targets`, then checks it.
    void applyRelocation(QWidget *parent, BitTorrent::Torrent *torrent, const Path &base, const QHash<int, Path> &targets)
    {
        QPointer<QWidget> guard {parent};
        const auto finish = [guard, torrent, targets]
        {
            pointAndCheck(guard, torrent, targets);
        };
        torrent->setAutoTMMEnabled(false);
        if (torrent->savePath() == base)
        {
            finish();
            return;
        }
        // Point the files once the new save path is in place.
        auto connection = std::make_shared<QMetaObject::Connection>();
        *connection = QObject::connect(BitTorrent::Session::instance(), &BitTorrent::Session::torrentSavePathChanged, BitTorrent::Session::instance()
            , [connection, torrent, finish](BitTorrent::Torrent *changed)
        {
            if (changed != torrent)
                return;
            QObject::disconnect(*connection);
            finish();
        });
        torrent->setSavePath(base);
    }

    void relocateTorrent(QWidget *parent, BitTorrent::Torrent *torrent)
    {
        if (blockedByExtension(parent))
            return;
        const QString folder = QFileDialog::getExistingDirectory(parent, tr("Choose the folder that now holds \"%1\"").arg(torrent->name())
            , torrent->savePath().toString());
        if (folder.isEmpty())
            return;
        const Path base {folder};
        const QHash<int, Path> targets = findFiles(torrent, base);
        if (targets.isEmpty())
        {
            QMessageBox::information(parent, title(), tr("None of this torrent's files are in that folder."));
            return;
        }
        const QString text = tr("Found %1 of %2 files in:\n%3\n\ntTorrent will look for the torrent there and check its files. "
            "Nothing is deleted; files that weren't found there are moved from the old folder if they're still in it.")
            .arg(QString::number(targets.size()), QString::number(torrent->filesCount()), QDir::toNativeSeparators(folder));
        if (QMessageBox::question(parent, title(), text, (QMessageBox::Ok | QMessageBox::Cancel), QMessageBox::Ok) != QMessageBox::Ok)
            return;

        applyRelocation(parent, torrent, base, targets);
    }

    // The folder path of a Content tab row, as the torrent names it ("Album/CD 1").
    QString rowPath(const QModelIndex &index)
    {
        QStringList parts;
        for (QModelIndex i = index; i.isValid(); i = i.parent())
            parts.prepend(i.data().toString());
        return parts.join(u'/');
    }

    void collectFiles(TorrentContentWidget *view, const QModelIndex &index, QList<int> &files)
    {
        const int file = view->getFileIndex(index);
        if (file >= 0)
        {
            files.append(file);
            return;
        }
        const QAbstractItemModel *model = view->model();
        for (int row = 0; row < model->rowCount(index); ++row)
            collectFiles(view, model->index(row, 0, index), files);
    }

    void relocateContent(TorrentContentWidget *view, BitTorrent::Torrent *torrent, const QModelIndex &index)
    {
        if (blockedByExtension(view))
            return;
        const int file = view->getFileIndex(index);
        QHash<int, Path> targets;
        if (file >= 0)
        {
            const QString chosen = QFileDialog::getOpenFileName(view, tr("Where is \"%1\" now?").arg(torrent->filePath(file).filename())
                , torrent->savePath().toString());
            if (chosen.isEmpty())
                return;
            targets.insert(file, Path(chosen));
        }
        else
        {
            const QString chosen = QFileDialog::getExistingDirectory(view, tr("Choose the folder that now holds \"%1\"").arg(index.data().toString())
                , torrent->savePath().toString());
            if (chosen.isEmpty())
                return;
            const QString prefix = rowPath(index) + u'/';
            QList<int> files;
            collectFiles(view, index, files);
            for (const int i : files)
            {
                const QString relative = torrent->filePath(i).data();
                if (!relative.startsWith(prefix))
                    continue;
                const Path target = Path(chosen) / Path(relative.mid(prefix.size()));
                if (target.exists())
                    targets.insert(i, target);
            }
            if (targets.isEmpty())
            {
                QMessageBox::information(view, title(), tr("None of the files in \"%1\" are in that folder.").arg(index.data().toString()));
                return;
            }
        }
        pointAndCheck(view, torrent, targets);
    }
}

QAction *DoseRelocate::addTorrentAction(QMenu *menu, QWidget *parent, const QList<BitTorrent::Torrent *> &torrents)
{
    if (!menu || (torrents.size() != 1) || !torrents.first()->hasMetadata())
        return nullptr;
    BitTorrent::Torrent *torrent = torrents.first();
    auto *action = menu->addAction(UIThemeManager::instance()->getIcon(u"set-location"_s), tr("Relocate..."), parent, [parent, torrent]
    {
        relocateTorrent(parent, torrent);
    });
    action->setToolTip(tr("Point this torrent at files you moved or renamed, then check them"));
    return action;
}

void DoseRelocate::addToContentMenu(QMenu *menu, TorrentContentWidget *view)
{
    if (!menu || !view)
        return;
    auto *torrent = dynamic_cast<BitTorrent::Torrent *>(view->contentHandler());
    const QModelIndexList rows = view->selectionModel()->selectedRows(0);
    if (!torrent || (rows.size() != 1))
        return;
    const QModelIndex index = rows.first();
    menu->addAction(UIThemeManager::instance()->getIcon(u"set-location"_s), tr("Relocate..."), view, [view, torrent, index]
    {
        relocateContent(view, torrent, index);
    });
}

int DoseRelocate::relocateTo(BitTorrent::Torrent *torrent, const QString &folder)
{
    if (!torrent || !torrent->hasMetadata())
        return 0;
    const Path base {folder};
    const QHash<int, Path> targets = findFiles(torrent, base);
    if (!targets.isEmpty())
        applyRelocation(nullptr, torrent, base, targets);
    return int(targets.size());
}
