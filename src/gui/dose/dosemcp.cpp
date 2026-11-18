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

#include "dosemcp.h"

#include <algorithm>
#include <functional>

#include <QApplication>
#include <QCheckBox>
#include <QClipboard>
#include <QComboBox>
#include <QCryptographicHash>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QPointer>
#include <QPushButton>
#include <QRandomGenerator>
#include <QStorageInfo>
#include <QTcpServer>
#include <QTcpSocket>
#include <QTimer>
#include <QUrl>

#include "base/bittorrent/addtorrentparams.h"
#include "base/bittorrent/infohash.h"
#include "base/bittorrent/session.h"
#include "base/bittorrent/sessionstatus.h"
#include "base/bittorrent/torrent.h"
#include "base/bittorrent/trackerentrystatus.h"
#include "base/global.h"
#include "base/logger.h"
#include "base/path.h"
#include "base/settingsstorage.h"
#include "base/ttorrentversion.h"
#include "base/utils/misc.h"
#include "base/utils/password.h"
#include "gui/guiaddtorrentmanager.h"
#include "gui/interfaces/iguiapplication.h"

namespace
{
    const QString KEY_ENABLED = u"tTorrent/McpEnabled"_s;
    const QString KEY_SCOPE = u"tTorrent/McpScope"_s;
    const QString KEY_HASH = u"tTorrent/McpKeyHash"_s;
    const QString MODERN = u"2026-07-28"_s;
    const QStringList LEGACY {u"2025-11-25"_s, u"2025-06-18"_s, u"2025-03-26"_s};
    const int MAX_BODY = 1024 * 1024;
    const int MAX_BULK = 20;

    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseMcp", text);
    }

    SettingsStorage *settings()
    {
        return SettingsStorage::instance();
    }

    QByteArray keyHash(const QString &key)
    {
        return QCryptographicHash::hash(key.toUtf8(), QCryptographicHash::Sha256).toHex();
    }

    // ---- Torrent data, shaped for agents -----------------------------------

    QString stateName(const BitTorrent::TorrentState state)
    {
        using S = BitTorrent::TorrentState;
        switch (state)
        {
        case S::ForcedDownloading: case S::Downloading: return u"downloading"_s;
        case S::ForcedDownloadingMetadata: case S::DownloadingMetadata: return u"fetching metadata"_s;
        case S::StalledDownloading: return u"stalled (downloading)"_s;
        case S::ForcedUploading: case S::Uploading: return u"seeding"_s;
        case S::StalledUploading: return u"seeding (idle)"_s;
        case S::CheckingResumeData: case S::CheckingUploading: case S::CheckingDownloading: return u"checking"_s;
        case S::QueuedDownloading: return u"queued (download)"_s;
        case S::QueuedUploading: return u"queued (seed)"_s;
        case S::StoppedDownloading: return u"stopped (incomplete)"_s;
        case S::StoppedUploading: return u"stopped (complete)"_s;
        case S::Moving: return u"moving"_s;
        case S::MissingFiles: return u"missing files"_s;
        case S::Error: return u"error"_s;
        default: return u"unknown"_s;
        }
    }

    QJsonObject sizeValue(const qint64 bytes)
    {
        return {{u"bytes"_s, bytes}, {u"text"_s, Utils::Misc::friendlyUnit(bytes)}};
    }

    // Announce URLs can hold a private tracker's passkey: keep the host only.
    QString maskedTracker(const QString &url)
    {
        const QUrl u {url};
        return u.isValid() ? (u.scheme() + u"://"_s + u.host() + ((u.port() > 0) ? u":"_s + QString::number(u.port()) : QString())) : u"(tracker)"_s;
    }

    QStringList tagNames(const BitTorrent::Torrent *torrent)
    {
        QStringList names;
        for (const Tag &tag : torrent->tags())
            names.append(tag.toString());
        return names;
    }

    QJsonObject brief(const BitTorrent::Torrent *t)
    {
        QJsonObject o {
            {u"id"_s, t->id().toString()},
            {u"name"_s, t->name()},
            {u"state"_s, stateName(t->state())},
            {u"progress"_s, qRound(t->progress() * 1000) / 10.0},
            {u"size"_s, sizeValue(t->wantedSize())},
            {u"download_speed"_s, sizeValue(t->downloadPayloadRate())},
            {u"upload_speed"_s, sizeValue(t->uploadPayloadRate())},
            {u"ratio"_s, qRound(t->realRatio() * 100) / 100.0},
            {u"category"_s, t->category()},
            {u"tags"_s, QJsonArray::fromStringList(tagNames(t))},
            {u"seeds"_s, t->seedsCount()},
            {u"peers"_s, t->peersCount()},
            {u"added"_s, t->addedTime().toString(Qt::ISODate)},
        };
        if (!t->isFinished() && (t->eta() < 8640000))
            o.insert(u"eta_seconds"_s, t->eta());
        if (t->hasError())
            o.insert(u"error"_s, t->error());
        return o;
    }

    bool matchesStatus(const BitTorrent::Torrent *t, const QString &status)
    {
        if (status.isEmpty() || (status == u"all"))
            return true;
        if (status == u"downloading")
            return t->isDownloading();
        if (status == u"seeding")
            return t->isFinished() && !t->isStopped();
        if (status == u"completed")
            return t->isFinished();
        if (status == u"stopped")
            return t->isStopped();
        if (status == u"active")
            return (t->downloadPayloadRate() > 0) || (t->uploadPayloadRate() > 0);
        if (status == u"stalled")
            return (t->state() == BitTorrent::TorrentState::StalledDownloading) || (t->state() == BitTorrent::TorrentState::StalledUploading);
        if (status == u"errored")
            return t->isErrored() || t->hasError() || (t->state() == BitTorrent::TorrentState::MissingFiles);
        return true;
    }

    QList<BitTorrent::Torrent *> resolveIds(const QJsonObject &args, QString *error)
    {
        QList<BitTorrent::Torrent *> list;
        for (const QJsonValue &v : args.value(u"ids"_s).toArray())
        {
            BitTorrent::Torrent *t = BitTorrent::Session::instance()->getTorrent(BitTorrent::TorrentID::fromString(v.toString()));
            if (!t)
            {
                *error = tr("No torrent with id %1.").arg(v.toString());
                return {};
            }
            list.append(t);
        }
        if (list.isEmpty())
            *error = tr("Give at least one torrent id in \"ids\" (from list_torrents).");
        else if (list.size() > MAX_BULK)
            *error = tr("At most %1 torrents per call; this change is too big to make without asking in the app.").arg(MAX_BULK);
        return list;
    }

    // ---- Tools --------------------------------------------------------------

    struct Tool
    {
        QString name;
        QString description;
        QJsonObject schema;
        bool manage;
        std::function<QJsonObject (const QJsonObject &, QString *)> run;
    };

    QJsonObject object(const QJsonObject &properties, const QStringList &required = {})
    {
        QJsonObject schema {{u"type"_s, u"object"_s}, {u"properties"_s, properties}};
        if (!required.isEmpty())
            schema.insert(u"required"_s, QJsonArray::fromStringList(required));
        return schema;
    }

    QJsonObject prop(const QString &type, const QString &description)
    {
        return {{u"type"_s, type}, {u"description"_s, description}};
    }

    QJsonObject idsProp()
    {
        return {{u"type"_s, u"array"_s}, {u"items"_s, QJsonObject {{u"type"_s, u"string"_s}}}
            , {u"description"_s, u"Torrent ids from list_torrents (at most 20)."_s}};
    }

    const QList<Tool> &tools()
    {
        static const QList<Tool> list {
            {u"get_status"_s, u"Overall state: speeds, counts by state, free disk space, whether tTorrent is reachable for incoming peers."_s, object({}), false
                , [](const QJsonObject &, QString *) -> QJsonObject
            {
                BitTorrent::Session *s = BitTorrent::Session::instance();
                const BitTorrent::SessionStatus &st = s->status();
                int downloading = 0, seeding = 0, stopped = 0, errored = 0;
                for (const BitTorrent::Torrent *t : s->torrents())
                {
                    if (t->isErrored() || t->hasError())
                        ++errored;
                    else if (t->isStopped())
                        ++stopped;
                    else if (t->isFinished())
                        ++seeding;
                    else
                        ++downloading;
                }
                const qint64 free = s->freeDiskSpace();
                QJsonObject o {
                    {u"torrents"_s, int(s->torrents().size())},
                    {u"downloading"_s, downloading}, {u"seeding"_s, seeding}, {u"stopped"_s, stopped}, {u"errored"_s, errored},
                    {u"download_speed"_s, sizeValue(st.payloadDownloadRate)},
                    {u"upload_speed"_s, sizeValue(st.payloadUploadRate)},
                    {u"incoming_connections"_s, st.hasIncomingConnections},
                    {u"free_disk_space"_s, sizeValue(free)},
                };
                o.insert(u"summary"_s, u"%1 torrents: %2 downloading, %3 seeding, %4 stopped, %5 with errors. Down %6/s, up %7/s. %8 free on the default drive."_s
                    .arg(QString::number(s->torrents().size()), QString::number(downloading), QString::number(seeding), QString::number(stopped)
                        , QString::number(errored), Utils::Misc::friendlyUnit(st.payloadDownloadRate), Utils::Misc::friendlyUnit(st.payloadUploadRate)
                        , Utils::Misc::friendlyUnit(free)));
                return o;
            }},
            {u"list_torrents"_s, u"Torrents, filtered and sorted. Returns id, name, state, progress %, size, speeds, ratio, category, tags."_s
                , object({
                    {u"status"_s, prop(u"string"_s, u"all, downloading, seeding, completed, stopped, active, stalled or errored."_s)},
                    {u"category"_s, prop(u"string"_s, u"Only this category."_s)},
                    {u"tag"_s, prop(u"string"_s, u"Only torrents with this tag."_s)},
                    {u"text"_s, prop(u"string"_s, u"Words that must all appear in the name."_s)},
                    {u"sort"_s, prop(u"string"_s, u"name, size, progress, ratio, added, download_speed or upload_speed (largest first except name)."_s)},
                    {u"limit"_s, prop(u"integer"_s, u"At most this many (default 50, max 500)."_s)},
                }), false
                , [](const QJsonObject &a, QString *) -> QJsonObject
            {
                const QStringList words = a.value(u"text"_s).toString().split(u' ', Qt::SkipEmptyParts);
                QList<BitTorrent::Torrent *> rows;
                for (BitTorrent::Torrent *t : BitTorrent::Session::instance()->torrents())
                {
                    if (!matchesStatus(t, a.value(u"status"_s).toString()))
                        continue;
                    if (a.contains(u"category"_s) && (t->category() != a.value(u"category"_s).toString()))
                        continue;
                    if (a.contains(u"tag"_s) && !tagNames(t).contains(a.value(u"tag"_s).toString()))
                        continue;
                    if (!std::all_of(words.cbegin(), words.cend(), [t](const QString &w) { return t->name().contains(w, Qt::CaseInsensitive); }))
                        continue;
                    rows.append(t);
                }
                const QString sort = a.value(u"sort"_s).toString(u"name"_s);
                std::sort(rows.begin(), rows.end(), [&sort](const BitTorrent::Torrent *x, const BitTorrent::Torrent *y)
                {
                    if (sort == u"size") return x->wantedSize() > y->wantedSize();
                    if (sort == u"progress") return x->progress() > y->progress();
                    if (sort == u"ratio") return x->realRatio() > y->realRatio();
                    if (sort == u"added") return x->addedTime() > y->addedTime();
                    if (sort == u"download_speed") return x->downloadPayloadRate() > y->downloadPayloadRate();
                    if (sort == u"upload_speed") return x->uploadPayloadRate() > y->uploadPayloadRate();
                    return x->name().compare(y->name(), Qt::CaseInsensitive) < 0;
                });
                const int limit = std::clamp(a.value(u"limit"_s).toInt(50), 1, 500);
                QJsonArray out;
                for (int i = 0; (i < rows.size()) && (i < limit); ++i)
                    out.append(brief(rows[i]));
                return {{u"total_matching"_s, int(rows.size())}, {u"torrents"_s, out}};
            }},
            {u"get_torrent"_s, u"Everything about one torrent: details, trackers (passkeys hidden), file count, save path."_s
                , object({{u"id"_s, prop(u"string"_s, u"Torrent id from list_torrents."_s)}}, {u"id"_s}), false
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                const BitTorrent::Torrent *t = BitTorrent::Session::instance()->getTorrent(BitTorrent::TorrentID::fromString(a.value(u"id"_s).toString()));
                if (!t)
                {
                    *error = tr("No torrent with that id.");
                    return {};
                }
                QJsonObject o = brief(t);
                o.insert(u"save_path"_s, t->savePath().toString());
                o.insert(u"files"_s, t->filesCount());
                o.insert(u"private"_s, t->isPrivate());
                o.insert(u"downloaded"_s, sizeValue(t->totalDownload()));
                o.insert(u"uploaded"_s, sizeValue(t->totalUpload()));
                o.insert(u"active_seconds"_s, t->activeTime());
                o.insert(u"seeds_in_swarm"_s, t->totalSeedsCount());
                o.insert(u"leechers_in_swarm"_s, t->totalLeechersCount());
                if (t->completedTime().isValid())
                    o.insert(u"completed"_s, t->completedTime().toString(Qt::ISODate));
                QJsonArray trackers;
                for (const BitTorrent::TrackerEntryStatus &tr : t->trackers())
                    trackers.append(QJsonObject {{u"tracker"_s, maskedTracker(tr.url)}, {u"message"_s, tr.message}});
                o.insert(u"trackers"_s, trackers);
                return o;
            }},
            {u"diagnose_torrent"_s, u"Why a torrent is slow or stuck: likely causes in order, each with a fix, in plain words."_s
                , object({{u"id"_s, prop(u"string"_s, u"Torrent id from list_torrents."_s)}}, {u"id"_s}), false
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                const BitTorrent::Torrent *t = BitTorrent::Session::instance()->getTorrent(BitTorrent::TorrentID::fromString(a.value(u"id"_s).toString()));
                if (!t)
                {
                    *error = tr("No torrent with that id.");
                    return {};
                }
                QJsonArray causes;
                const auto add = [&causes](const QString &cause, const QString &fix) { causes.append(QJsonObject {{u"cause"_s, cause}, {u"fix"_s, fix}}); };
                if (t->hasError() || t->isErrored())
                    add(u"The torrent has an error: "_s + t->error(), u"Fix the cause (often a missing drive or no permission to write), then recheck."_s);
                if (t->state() == BitTorrent::TorrentState::MissingFiles)
                    add(u"Its files are missing from the save path."_s, u"Reconnect the drive or move the files back, then recheck; or set the right location."_s);
                if (t->isStopped())
                    add(u"It is stopped."_s, u"Start it."_s);
                const qint64 free = QStorageInfo(t->savePath().data()).bytesAvailable();
                if (!t->isFinished() && (free >= 0) && (free < (t->wantedSize() - qint64(t->progress() * t->wantedSize()))))
                    add(u"Not enough free space for the rest of the download."_s, u"Free up space on the drive or move the torrent."_s);
                if (!t->isFinished() && (t->totalSeedsCount() == 0))
                    add(u"No one in the swarm has the whole torrent (0 seeds known)."_s, u"Wait for seeds, or find another source; it may never finish."_s);
                int working = 0;
                for (const BitTorrent::TrackerEntryStatus &tr : t->trackers())
                {
                    if (tr.message.isEmpty())
                        ++working;
                }
                if (!t->trackers().isEmpty() && (working == 0))
                    add(u"Every tracker reported a problem."_s, u"Check the tracker messages in get_torrent; add working trackers or rely on DHT."_s);
                if (!BitTorrent::Session::instance()->status().hasIncomingConnections)
                    add(u"No incoming connections: other peers can't connect to tTorrent (port not forwarded)."_s, u"Forward the listening port on the router or turn on UPnP in Settings › Connection."_s);
                if (causes.isEmpty())
                    add(u"Nothing looks wrong. Speed depends on how many peers have the pieces and their upload speeds."_s, u"Give it time."_s);
                return {{u"torrent"_s, t->name()}, {u"state"_s, stateName(t->state())}, {u"causes"_s, causes}};
            }},
            {u"list_categories"_s, u"Categories with how many torrents each has, and tags with counts."_s, object({}), false
                , [](const QJsonObject &, QString *) -> QJsonObject
            {
                QHash<QString, int> categories, tags;
                for (const BitTorrent::Torrent *t : BitTorrent::Session::instance()->torrents())
                {
                    ++categories[t->category()];
                    for (const QString &tag : tagNames(t))
                        ++tags[tag];
                }
                QJsonArray c, g;
                for (const QString &name : BitTorrent::Session::instance()->categories())
                    c.append(QJsonObject {{u"name"_s, name}, {u"torrents"_s, categories.value(name)}});
                for (const Tag &tag : BitTorrent::Session::instance()->tags())
                    g.append(QJsonObject {{u"name"_s, tag.toString()}, {u"torrents"_s, tags.value(tag.toString())}});
                return {{u"categories"_s, c}, {u"uncategorized"_s, categories.value(QString())}, {u"tags"_s, g}};
            }},
            {u"plan_disk_cleanup"_s, u"Proposes finished torrents to remove to free a target amount: largest and best-seeded first, private-tracker torrents with a ratio under 1 last. Changes nothing."_s
                , object({{u"target_gb"_s, prop(u"number"_s, u"How much space to free, in GB."_s)}}, {u"target_gb"_s}), false
                , [](const QJsonObject &a, QString *) -> QJsonObject
            {
                const qint64 target = qint64(a.value(u"target_gb"_s).toDouble() * 1e9);
                QList<BitTorrent::Torrent *> done;
                for (BitTorrent::Torrent *t : BitTorrent::Session::instance()->torrents())
                {
                    if (t->isFinished())
                        done.append(t);
                }
                std::sort(done.begin(), done.end(), [](const BitTorrent::Torrent *x, const BitTorrent::Torrent *y)
                {
                    const bool xKeep = x->isPrivate() && (x->realRatio() < 1.0);
                    const bool yKeep = y->isPrivate() && (y->realRatio() < 1.0);
                    if (xKeep != yKeep)
                        return !xKeep;
                    return x->wantedSize() > y->wantedSize();
                });
                QJsonArray plan;
                qint64 freed = 0;
                for (const BitTorrent::Torrent *t : done)
                {
                    if (freed >= target)
                        break;
                    QJsonObject o = brief(t);
                    o.insert(u"why"_s, (t->isPrivate() && (t->realRatio() < 1.0)) ? u"Private tracker with ratio under 1: removing may hurt your standing."_s
                        : u"Finished, ratio %1."_s.arg(t->realRatio(), 0, 'f', 2));
                    plan.append(o);
                    freed += t->wantedSize();
                }
                return {{u"would_free"_s, sizeValue(freed)}, {u"torrents"_s, plan}
                    , {u"note"_s, u"Nothing was removed. Ask the person before calling remove_torrents (which keeps the files; deleting files is not allowed for agents)."_s}};
            }},
            {u"add_torrent"_s, u"Adds a torrent from a magnet link, a web link to a .torrent file, or a path to a .torrent file on this computer."_s
                , object({
                    {u"source"_s, prop(u"string"_s, u"Magnet link, http(s) link or local file path."_s)},
                    {u"category"_s, prop(u"string"_s, u"Category to put it in."_s)},
                    {u"save_path"_s, prop(u"string"_s, u"Folder to download into (default: the usual one)."_s)},
                    {u"stopped"_s, prop(u"boolean"_s, u"Add without starting."_s)},
                }, {u"source"_s}), true
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                auto *app = dynamic_cast<IGUIApplication *>(QCoreApplication::instance());
                if (!app)
                {
                    *error = tr("tTorrent can't add torrents right now.");
                    return {};
                }
                BitTorrent::AddTorrentParams params;
                params.category = a.value(u"category"_s).toString();
                if (a.contains(u"save_path"_s))
                {
                    params.savePath = Path(a.value(u"save_path"_s).toString());
                    params.useAutoTMM = false;
                }
                if (a.contains(u"stopped"_s))
                    params.addStopped = a.value(u"stopped"_s).toBool();
                const QString source = a.value(u"source"_s).toString().trimmed();
                if (!app->addTorrentManager()->addTorrent(source, params, AddTorrentOption::SkipDialog))
                {
                    *error = tr("Couldn't add that source. Check the magnet link, web link or file path.");
                    return {};
                }
                return {{u"added"_s, true}, {u"summary"_s, u"Adding it now; call list_torrents with text from its name to see it."_s}};
            }},
            {u"stop_torrents"_s, u"Stops (pauses) torrents."_s, object({{u"ids"_s, idsProp()}}, {u"ids"_s}), true
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                const auto list = resolveIds(a, error);
                for (BitTorrent::Torrent *t : list)
                    t->stop();
                return {{u"stopped"_s, int(list.size())}};
            }},
            {u"start_torrents"_s, u"Starts (resumes) torrents."_s, object({{u"ids"_s, idsProp()}}, {u"ids"_s}), true
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                const auto list = resolveIds(a, error);
                for (BitTorrent::Torrent *t : list)
                    t->start();
                return {{u"started"_s, int(list.size())}};
            }},
            {u"recheck_torrents"_s, u"Checks the files of torrents against their hashes (reads the whole files)."_s, object({{u"ids"_s, idsProp()}}, {u"ids"_s}), true
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                const auto list = resolveIds(a, error);
                for (BitTorrent::Torrent *t : list)
                    t->forceRecheck();
                return {{u"rechecking"_s, int(list.size())}};
            }},
            {u"reannounce_torrents"_s, u"Asks the trackers of torrents for peers again now."_s, object({{u"ids"_s, idsProp()}}, {u"ids"_s}), true
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                const auto list = resolveIds(a, error);
                for (BitTorrent::Torrent *t : list)
                    t->forceReannounce();
                return {{u"reannounced"_s, int(list.size())}};
            }},
            {u"set_category"_s, u"Puts torrents in a category (empty removes it). The category must exist (see list_categories)."_s
                , object({{u"ids"_s, idsProp()}, {u"category"_s, prop(u"string"_s, u"Category name, or empty."_s)}}, {u"ids"_s, u"category"_s}), true
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                const auto list = resolveIds(a, error);
                int changed = 0;
                for (BitTorrent::Torrent *t : list)
                    changed += t->setCategory(a.value(u"category"_s).toString()) ? 1 : 0;
                if (!list.isEmpty() && (changed == 0))
                    *error = tr("That category doesn't exist.");
                return {{u"changed"_s, changed}};
            }},
            {u"remove_torrents"_s, u"Removes torrents from tTorrent. Their downloaded files stay on disk (agents can't delete files)."_s
                , object({{u"ids"_s, idsProp()}}, {u"ids"_s}), true
                , [](const QJsonObject &a, QString *error) -> QJsonObject
            {
                const auto list = resolveIds(a, error);
                QStringList names;
                for (BitTorrent::Torrent *t : list)
                {
                    names.append(t->name());
                    BitTorrent::Session::instance()->removeTorrent(t->id(), BitTorrent::TorrentRemoveOption::KeepContent);
                }
                if (!names.isEmpty())
                    LogMsg(tr("An AI agent removed %1 torrent(s) (files kept): %2").arg(QString::number(names.size()), names.join(u", "_s)));
                return {{u"removed"_s, int(names.size())}, {u"files_kept"_s, true}};
            }},
        };
        return list;
    }

    // ---- JSON-RPC over HTTP ------------------------------------------------

    struct Reply
    {
        int status = 200;
        QByteArray body;
    };

    Reply jsonReply(const QJsonObject &message, const int status = 200)
    {
        return {status, QJsonDocument(message).toJson(QJsonDocument::Compact)};
    }

    Reply error(const QJsonValue &id, const int code, const QString &message, const int status = 200, const QJsonObject &data = {})
    {
        QJsonObject err {{u"code"_s, code}, {u"message"_s, message}};
        if (!data.isEmpty())
            err.insert(u"data"_s, data);
        QJsonObject msg {{u"jsonrpc"_s, u"2.0"_s}, {u"error"_s, err}};
        if (!id.isUndefined())
            msg.insert(u"id"_s, id);
        return jsonReply(msg, status);
    }

    Reply result(const QJsonValue &id, QJsonObject value)
    {
        value.insert(u"resultType"_s, u"complete"_s);
        return jsonReply({{u"jsonrpc"_s, u"2.0"_s}, {u"id"_s, id}, {u"result"_s, value}});
    }

    QString decodeHeader(const QString &value)
    {
        if (value.startsWith(u"=?base64?") && value.endsWith(u"?="))
            return QString::fromUtf8(QByteArray::fromBase64(value.mid(9, value.size() - 11).toLatin1()));
        return value;
    }

    QJsonObject serverInfo()
    {
        return {{u"name"_s, u"tTorrent"_s}, {u"version"_s, QStringLiteral(TTORRENT_VERSION)}};
    }

    QString instructions()
    {
        return u"tTorrent is a BitTorrent client. Use get_status for an overview and list_torrents to find torrent ids. "
            "Explain problems with diagnose_torrent. Before removing anything, show the person plan_disk_cleanup's list and ask. "
            "Removing never deletes downloaded files."_s;
    }

    Reply handle(const QHash<QString, QString> &headers, const QByteArray &body)
    {
        QJsonParseError parseError;
        const QJsonDocument doc = QJsonDocument::fromJson(body, &parseError);
        if (!doc.isObject())
            return error(QJsonValue::Undefined, -32700, u"Parse error"_s, 400);
        const QJsonObject msg = doc.object();
        const QString method = msg.value(u"method"_s).toString();
        const QJsonValue id = msg.value(u"id"_s);
        const QJsonObject params = msg.value(u"params"_s).toObject();
        const bool notification = id.isUndefined();

        if (notification)
            return {202, {}};

        const QString version = params.value(u"_meta"_s).toObject().value(u"io.modelcontextprotocol/protocolVersion"_s).toString();
        const bool modern = !version.isEmpty();
        if (modern)
        {
            if (version != MODERN)
            {
                return error(id, -32022, u"Unsupported protocol version"_s, 400
                    , {{u"supported"_s, QJsonArray {MODERN, LEGACY[0], LEGACY[1], LEGACY[2]}}, {u"requested"_s, version}});
            }
            // The headers mirror the body; they must agree.
            if (headers.value(u"mcp-protocol-version"_s) != version)
                return error(id, -32020, u"Header mismatch: MCP-Protocol-Version"_s, 400);
            if (headers.value(u"mcp-method"_s) != method)
                return error(id, -32020, u"Header mismatch: Mcp-Method"_s, 400);
            if ((method == u"tools/call") && (decodeHeader(headers.value(u"mcp-name"_s)) != params.value(u"name"_s).toString()))
                return error(id, -32020, u"Header mismatch: Mcp-Name"_s, 400);
        }

        const bool manage = (settings()->loadValue<QString>(KEY_SCOPE, u"read"_s) == u"manage");
        if (method == u"initialize")
        {
            const QString wanted = params.value(u"protocolVersion"_s).toString();
            return result(id, {
                {u"protocolVersion"_s, LEGACY.contains(wanted) ? wanted : LEGACY.first()},
                {u"capabilities"_s, QJsonObject {{u"tools"_s, QJsonObject {}}}},
                {u"serverInfo"_s, serverInfo()},
                {u"instructions"_s, instructions()},
            });
        }
        if (method == u"server/discover")
        {
            return result(id, {
                {u"supportedVersions"_s, QJsonArray {MODERN, LEGACY[0], LEGACY[1], LEGACY[2]}},
                {u"capabilities"_s, QJsonObject {{u"tools"_s, QJsonObject {}}}},
                {u"_meta"_s, QJsonObject {{u"io.modelcontextprotocol/serverInfo"_s, serverInfo()}}},
                {u"instructions"_s, instructions()},
            });
        }
        if (method == u"ping")
            return result(id, {});
        if (method == u"tools/list")
        {
            QJsonArray list;
            for (const Tool &tool : tools())
            {
                if (tool.manage && !manage)
                    continue;
                QJsonObject t {{u"name"_s, tool.name}, {u"description"_s, tool.description}, {u"inputSchema"_s, tool.schema}};
                t.insert(u"annotations"_s, QJsonObject {{u"readOnlyHint"_s, !tool.manage}, {u"destructiveHint"_s, tool.name == u"remove_torrents"}});
                list.append(t);
            }
            return result(id, {{u"tools"_s, list}});
        }
        if (method == u"tools/call")
        {
            const QString name = params.value(u"name"_s).toString();
            const auto it = std::find_if(tools().cbegin(), tools().cend(), [&name](const Tool &t) { return t.name == name; });
            if ((it == tools().cend()) || (it->manage && !manage))
                return error(id, -32602, u"Unknown tool: %1"_s.arg(name));
            QString message;
            const QJsonObject value = it->run(params.value(u"arguments"_s).toObject(), &message);
            const QString text = message.isEmpty() ? QString::fromUtf8(QJsonDocument(value).toJson(QJsonDocument::Indented)) : message;
            QJsonObject out {{u"content"_s, QJsonArray {QJsonObject {{u"type"_s, u"text"_s}, {u"text"_s, text}}}}, {u"isError"_s, !message.isEmpty()}};
            if (message.isEmpty())
                out.insert(u"structuredContent"_s, value);
            return result(id, out);
        }
        return error(id, -32601, u"Method not found"_s, modern ? 404 : 200);
    }

    // ---- The listener -------------------------------------------------------

    class Server final : public QTcpServer
    {
    public:
        using QTcpServer::QTcpServer;

    protected:
        void incomingConnection(const qintptr descriptor) override
        {
            auto *socket = new QTcpSocket(this);
            if (!socket->setSocketDescriptor(descriptor) || !socket->peerAddress().isLoopback())
            {
                socket->deleteLater();
                return;
            }
            auto *buffer = new QByteArray;
            QTimer::singleShot(30000, socket, [socket] { socket->abort(); });
            connect(socket, &QObject::destroyed, socket, [buffer] { delete buffer; });
            connect(socket, &QTcpSocket::disconnected, socket, &QObject::deleteLater);
            connect(socket, &QTcpSocket::readyRead, socket, [socket, buffer]
            {
                buffer->append(socket->readAll());
                if (buffer->size() > (MAX_BODY + 16384))
                {
                    respond(socket, 413, {});
                    return;
                }
                const qsizetype end = buffer->indexOf("\r\n\r\n");
                if (end < 0)
                    return;
                const QList<QByteArray> lines = buffer->left(end).split('\n');
                const QList<QByteArray> requestLine = lines.value(0).trimmed().split(' ');
                QHash<QString, QString> headers;
                for (qsizetype i = 1; i < lines.size(); ++i)
                {
                    const qsizetype colon = lines[i].indexOf(':');
                    if (colon > 0)
                        headers.insert(QString::fromLatin1(lines[i].left(colon)).trimmed().toLower(), QString::fromUtf8(lines[i].mid(colon + 1)).trimmed());
                }
                const qsizetype length = headers.value(u"content-length"_s).toLongLong();
                if (buffer->size() < (end + 4 + length))
                    return;
                const QByteArray body = buffer->mid(end + 4, length);
                serve(socket, QString::fromLatin1(requestLine.value(0)), QString::fromLatin1(requestLine.value(1)), headers, body);
            });
        }

    private:
        static void respond(QTcpSocket *socket, const int status, const QByteArray &body)
        {
            static const QHash<int, QByteArray> texts {{200, "OK"}, {202, "Accepted"}, {400, "Bad Request"}, {401, "Unauthorized"}
                , {403, "Forbidden"}, {404, "Not Found"}, {405, "Method Not Allowed"}, {413, "Payload Too Large"}};
            QByteArray head = "HTTP/1.1 " + QByteArray::number(status) + ' ' + texts.value(status, "Error") + "\r\n";
            if (!body.isEmpty())
                head += "Content-Type: application/json\r\n";
            if (status == 401)
                head += "WWW-Authenticate: Bearer\r\n";
            if (status == 405)
                head += "Allow: POST\r\n";
            head += "Content-Length: " + QByteArray::number(body.size()) + "\r\nConnection: close\r\n\r\n";
            socket->write(head + body);
            socket->disconnectFromHost();
        }

        static void serve(QTcpSocket *socket, const QString &method, const QString &path, const QHash<QString, QString> &headers, const QByteArray &body)
        {
            if (QUrl(path).path() != u"/mcp")
                return respond(socket, 404, {});
            // DNS rebinding: a web page may only reach us from a local origin.
            if (const QString origin = headers.value(u"origin"_s); !origin.isEmpty())
            {
                const QString host = QUrl(origin).host();
                if ((host != u"127.0.0.1") && (host != u"localhost") && (host != u"[::1]") && (host != u"::1"))
                    return respond(socket, 403, {});
            }
            if (method != u"POST")
                return respond(socket, 405, {});
            const QString auth = headers.value(u"authorization"_s);
            const QByteArray stored = settings()->loadValue<QByteArray>(KEY_HASH);
            if (stored.isEmpty() || !auth.startsWith(u"Bearer ", Qt::CaseInsensitive)
                || !Utils::Password::slowEquals(keyHash(auth.mid(7).trimmed()), stored))
            {
                return respond(socket, 401, {});
            }
            const Reply reply = handle(headers, body);
            respond(socket, reply.status, reply.body);
        }
    };

    QPointer<Server> &server()
    {
        static QPointer<Server> instance;
        return instance;
    }

    QString newKey()
    {
        QByteArray bytes(24, Qt::Uninitialized);
        QRandomGenerator::system()->fillRange(reinterpret_cast<quint32 *>(bytes.data()), bytes.size() / 4);
        return u"tt_"_s + QString::fromLatin1(bytes.toBase64(QByteArray::Base64UrlEncoding | QByteArray::OmitTrailingEquals));
    }

    QString endpoint()
    {
        return u"http://127.0.0.1:%1/mcp"_s.arg(DoseMcp::PORT);
    }
}

void DoseMcp::apply()
{
    const bool on = settings()->loadValue<bool>(KEY_ENABLED, false) && !settings()->loadValue<QByteArray>(KEY_HASH).isEmpty();
    if (!on)
    {
        if (server())
        {
            server()->close();
            server()->deleteLater();
            LogMsg(tr("AI agent access (MCP) turned off."));
        }
        return;
    }
    if (server() && server()->isListening())
        return;
    server() = new Server(qApp);
    if (server()->listen(QHostAddress::LocalHost, PORT))
        LogMsg(tr("AI agents can connect at %1 (MCP).").arg(endpoint()));
    else
        LogMsg(tr("AI agent access (MCP) couldn't start on port %1: %2").arg(QString::number(PORT), server()->errorString()), Log::WARNING);
}

QWidget *DoseMcp::settingsGroup()
{
    auto *group = new QGroupBox(tr("AI agents (MCP)"));
    group->setObjectName(u"doseMcp"_s);
    auto *grid = new QGridLayout(group);
    int row = 0;

    auto *enable = new QCheckBox(tr("Let AI agents (Claude Code, Codex, Cursor) connect on this computer"));
    enable->setObjectName(u"doseMcpEnabled"_s);
    enable->setChecked(settings()->loadValue<bool>(KEY_ENABLED, false));
    grid->addWidget(enable, row++, 0, 1, 3);

    grid->addWidget(new QLabel(tr("Agents may:")), row, 0);
    auto *scope = new QComboBox;
    scope->setObjectName(u"doseMcpScope"_s);
    scope->addItem(tr("Look only"), u"read"_s);
    scope->addItem(tr("Look and manage (add, stop, start, categorise, remove without files)"), u"manage"_s);
    scope->setCurrentIndex(std::max(0, scope->findData(settings()->loadValue<QString>(KEY_SCOPE, u"read"_s))));
    grid->addWidget(scope, row++, 1, 1, 2);

    grid->addWidget(new QLabel(tr("Key:")), row, 0);
    auto *key = new QLineEdit;
    key->setObjectName(u"doseMcpKey"_s);
    key->setReadOnly(true);
    key->setPlaceholderText(settings()->loadValue<QByteArray>(KEY_HASH).isEmpty() ? tr("No key yet: create one") : tr("Set (shown only when created)"));
    auto *create = new QPushButton(tr("New key"));
    create->setAutoDefault(false);
    grid->addWidget(key, row, 1);
    grid->addWidget(create, row++, 2);

    auto *command = new QLineEdit;
    command->setObjectName(u"doseMcpCommand"_s);
    command->setReadOnly(true);
    command->setVisible(false);
    auto *copy = new QPushButton(tr("Copy"));
    copy->setAutoDefault(false);
    copy->setVisible(false);
    auto *commandLabel = new QLabel(tr("Claude Code:"));
    commandLabel->setVisible(false);
    grid->addWidget(commandLabel, row, 0);
    grid->addWidget(command, row, 1);
    grid->addWidget(copy, row++, 2);

    auto *note = new QLabel(tr("Address: %1, this computer only. Agents never see passwords or tracker passkeys, and can't delete downloaded files.").arg(endpoint()));
    note->setWordWrap(true);
    note->setObjectName(u"doseMuted"_s);
    grid->addWidget(note, row++, 0, 1, 3);
    grid->setColumnStretch(1, 1);

    QObject::connect(create, &QPushButton::clicked, group, [group, key, command, copy, commandLabel]
    {
        const QString fresh = newKey();
        group->setProperty("doseMcpNewKey", fresh);
        key->setText(fresh);
        command->setText(u"claude mcp add --transport http ttorrent %1 --header \"Authorization: Bearer %2\""_s.arg(endpoint(), fresh));
        command->setVisible(true);
        copy->setVisible(true);
        commandLabel->setVisible(true);
    });
    QObject::connect(copy, &QPushButton::clicked, command, [command, copy]
    {
        QApplication::clipboard()->setText(command->text());
        copy->setText(tr("Copied"));
    });
    return group;
}

void DoseMcp::saveSettings(QWidget *group)
{
    if (!group)
        return;
    if (auto *enable = group->findChild<QCheckBox *>(u"doseMcpEnabled"_s))
        settings()->storeValue(KEY_ENABLED, enable->isChecked());
    if (auto *scope = group->findChild<QComboBox *>(u"doseMcpScope"_s))
        settings()->storeValue(KEY_SCOPE, scope->currentData().toString());
    if (const QString fresh = group->property("doseMcpNewKey").toString(); !fresh.isEmpty())
    {
        settings()->storeValue(KEY_HASH, keyHash(fresh));
        group->setProperty("doseMcpNewKey", QString());
    }
    apply();
}
