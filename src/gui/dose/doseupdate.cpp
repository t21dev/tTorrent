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

#include "doseupdate.h"

#include <QAction>
#include <QCheckBox>
#include <QDateTime>
#include <QDesktopServices>
#include <QGroupBox>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QMainWindow>
#include <QMenu>
#include <QMessageBox>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QPointer>
#include <QRegularExpression>
#include <QTimer>
#include <QUrl>
#include <QVBoxLayout>
#include <QXmlStreamReader>

#include "base/global.h"
#include "base/logger.h"
#include "base/settingsstorage.h"
#include "base/ttorrentversion.h"
#include "doseautomation.h"
#include "gui/uithememanager.h"

namespace
{
    const QString KEY_AUTO = u"tTorrent/UpdateCheck"_s;
    const QString KEY_PREVIEWS = u"tTorrent/UpdatePreviews"_s;
    const QString KEY_LAST = u"tTorrent/UpdateLastCheck"_s;
    const QString KEY_SKIPPED = u"tTorrent/UpdateSkipped"_s;
    const QString DEFAULT_REPO = u"t21dev/tTorrent"_s;

    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseUpdate", text);
    }

    SettingsStorage *settings()
    {
        return SettingsStorage::instance();
    }

    QString repository()
    {
        const QString override = qEnvironmentVariable("TTORRENT_UPDATE_REPO");
        return override.isEmpty() ? DEFAULT_REPO : override;
    }

    // "v0.3.0", "release-5.3.0" and "0.3.0-rc1" all become {0, 3, 0}.
    QList<int> versionOf(const QString &text)
    {
        static const QRegularExpression pattern {u"(\\d+)\\.(\\d+)(?:\\.(\\d+))?"_s};
        const QRegularExpressionMatch m = pattern.match(text);
        if (!m.hasMatch())
            return {};
        return {m.captured(1).toInt(), m.captured(2).toInt(), m.captured(3).isEmpty() ? 0 : m.captured(3).toInt()};
    }

    bool newer(const QList<int> &candidate, const QList<int> &current)
    {
        return !candidate.isEmpty() && (candidate > current);
    }

    struct Release
    {
        QString tag;
        QString name;
        QUrl page;
        QUrl download;
        QDateTime published;
    };

    // Fallback: the public Atom feed of releases. It doesn't say which are
    // previews, so they all count; the zip's address follows our naming.
    void fetchFeed(QObject *context, std::function<void (const Release &, const QString &)> done)
    {
        auto *network = new QNetworkAccessManager(context);
        QNetworkRequest request {QUrl(u"https://github.com/%1/releases.atom"_s.arg(repository()))};
        request.setHeader(QNetworkRequest::UserAgentHeader, u"tTorrent/%1"_s.arg(QStringLiteral(TTORRENT_VERSION)));
        request.setTransferTimeout(15000);
        QNetworkReply *reply = network->get(request);
        QObject::connect(reply, &QNetworkReply::finished, context, [reply, network, done]
        {
            reply->deleteLater();
            network->deleteLater();
            const int status = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
            if (reply->error() != QNetworkReply::NoError)
            {
                done({}, (status == 404) ? tr("the release list isn't public yet") : reply->errorString());
                return;
            }
            const QList<int> current = versionOf(QStringLiteral(TTORRENT_VERSION));
            Release best;
            QList<int> bestVersion = current;
            QXmlStreamReader xml {reply->readAll()};
            QString title, link, updated;
            while (!xml.atEnd())
            {
                xml.readNext();
                if (xml.isStartElement() && (xml.name() == u"entry"))
                {
                    title.clear();
                    link.clear();
                    updated.clear();
                }
                else if (xml.isStartElement() && (xml.name() == u"link"))
                {
                    link = xml.attributes().value(u"href"_s).toString();
                }
                else if (xml.isStartElement() && (xml.name() == u"title"))
                {
                    title = xml.readElementText();
                }
                else if (xml.isStartElement() && (xml.name() == u"updated"))
                {
                    updated = xml.readElementText();
                }
                else if (xml.isEndElement() && (xml.name() == u"entry"))
                {
                    const QString tag = link.section(u'/', -1);
                    const QList<int> version = versionOf(tag);
                    if (!newer(version, bestVersion))
                        continue;
                    bestVersion = version;
                    const QString plain = u"%1.%2.%3"_s.arg(version[0]).arg(version[1]).arg(version[2]);
                    best = {tag, title, QUrl(link)
                        , QUrl(u"https://github.com/%1/releases/download/%2/tTorrent-%3-windows-x64.zip"_s.arg(repository(), tag, plain))
                        , QDateTime::fromString(updated, Qt::ISODate)};
                }
            }
            done(best, {});
        });
    }

    // Asks GitHub for the newest releases and calls `done` with the newest one
    // above this version (empty tag if none) or an error message.
    void fetch(QObject *context, std::function<void (const Release &, const QString &)> done)
    {
        auto *network = new QNetworkAccessManager(context);
        QNetworkRequest request {QUrl(u"https://api.github.com/repos/%1/releases?per_page=10"_s.arg(repository()))};
        request.setRawHeader("Accept", "application/vnd.github+json");
        request.setRawHeader("X-GitHub-Api-Version", "2022-11-28");
        request.setHeader(QNetworkRequest::UserAgentHeader, u"tTorrent/%1"_s.arg(QStringLiteral(TTORRENT_VERSION)));
        request.setTransferTimeout(15000);
        QNetworkReply *reply = network->get(request);
        QObject::connect(reply, &QNetworkReply::finished, context, [context, reply, network, done]
        {
            reply->deleteLater();
            network->deleteLater();
            const int status = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
            if ((status == 403) || (status == 429))
            {
                // The API allows 60 requests an hour per address; the feed isn't limited that way.
                fetchFeed(context, done);
                return;
            }
            if (reply->error() != QNetworkReply::NoError)
            {
                done({}, (status == 404) ? tr("the release list isn't public yet") : reply->errorString());
                return;
            }
            const bool previews = settings()->loadValue<bool>(KEY_PREVIEWS, true);
            const QList<int> current = versionOf(QStringLiteral(TTORRENT_VERSION));
            Release best;
            QList<int> bestVersion = current;
            for (const QJsonValue &value : QJsonDocument::fromJson(reply->readAll()).array())
            {
                const QJsonObject r = value.toObject();
                if (r.value(u"draft"_s).toBool() || (r.value(u"prerelease"_s).toBool() && !previews))
                    continue;
                const QList<int> version = versionOf(r.value(u"tag_name"_s).toString());
                if (!newer(version, bestVersion))
                    continue;
                bestVersion = version;
                best = {r.value(u"tag_name"_s).toString(), r.value(u"name"_s).toString(), QUrl(r.value(u"html_url"_s).toString()), {}
                    , QDateTime::fromString(r.value(u"published_at"_s).toString(), Qt::ISODate)};
                for (const QJsonValue &asset : r.value(u"assets"_s).toArray())
                {
                    const QString name = asset.toObject().value(u"name"_s).toString();
                    if (name.endsWith(u"-windows-x64.zip"))
                        best.download = QUrl(asset.toObject().value(u"browser_download_url"_s).toString());
                }
            }
            done(best, {});
        });
    }

    void offer(QMainWindow *window, const Release &release)
    {
        const QString version = versionOf(release.tag).isEmpty() ? release.tag
            : u"%1.%2.%3"_s.arg(versionOf(release.tag)[0]).arg(versionOf(release.tag)[1]).arg(versionOf(release.tag)[2]);
        const QUrl download = release.download.isValid() ? release.download : release.page;
        const QUrl page = release.page;
        const QString tag = release.tag;
        DoseAutomation::showCard(window, tr("tTorrent %1 is available").arg(version)
            , tr("You have %1. Download the zip, unzip it over this copy and restart tTorrent.").arg(QStringLiteral(TTORRENT_VERSION)), u"download"_s
            , {{tr("What's new"), [page] { QDesktopServices::openUrl(page); }},
               {tr("Download"), [download] { QDesktopServices::openUrl(download); }},
               {tr("Skip this version"), [tag] { settings()->storeValue(KEY_SKIPPED, tag); }}}, true);
    }

    void automaticCheck(QMainWindow *window)
    {
        if (!settings()->loadValue<bool>(KEY_AUTO, true) || qEnvironmentVariableIsSet("TTORRENT_UISHOT_DIR"))
            return;
        const QDateTime last = settings()->loadValue<QDateTime>(KEY_LAST);
        if (last.isValid() && (last.secsTo(QDateTime::currentDateTimeUtc()) < (20 * 60 * 60)))
            return;
        settings()->storeValue(KEY_LAST, QDateTime::currentDateTimeUtc());
        QPointer<QMainWindow> guard {window};
        fetch(window, [guard](const Release &release, const QString &error)
        {
            if (!error.isEmpty())
            {
                LogMsg(tr("Couldn't check for tTorrent updates: %1.").arg(error), Log::INFO);
                return;
            }
            if (!guard || release.tag.isEmpty() || (release.tag == settings()->loadValue<QString>(KEY_SKIPPED)))
                return;
            LogMsg(tr("tTorrent %1 is available.").arg(release.tag));
            offer(guard, release);
        });
    }
}

void DoseUpdate::checkNow(QMainWindow *window)
{
    settings()->storeValue(KEY_LAST, QDateTime::currentDateTimeUtc());
    QPointer<QMainWindow> guard {window};
    fetch(window, [guard](const Release &release, const QString &error)
    {
        if (!guard)
            return;
        if (!error.isEmpty())
        {
            QMessageBox::information(guard, tr("Check for updates"), tr("Couldn't check for updates: %1.").arg(error));
            return;
        }
        if (release.tag.isEmpty())
        {
            QMessageBox::information(guard, tr("Check for updates"), tr("You have the latest version, tTorrent %1.").arg(QStringLiteral(TTORRENT_VERSION)));
            return;
        }
        settings()->removeValue(KEY_SKIPPED);
        offer(guard, release);
    });
}

void DoseUpdate::install(QMainWindow *window)
{
    if (!window)
        return;
    if (QMenu *help = window->findChild<QMenu *>(u"menuHelp"_s))
    {
        auto *action = new QAction(UIThemeManager::instance()->getIcon(u"view-refresh"_s), tr("Check for updates…"), window);
        action->setObjectName(u"doseActionUpdates"_s);
        QObject::connect(action, &QAction::triggered, window, [window] { checkNow(window); });
        QAction *about = window->findChild<QAction *>(u"actionAbout"_s);
        help->insertAction((about && help->actions().contains(about)) ? about : nullptr, action);
    }
    QTimer::singleShot(60 * 1000, window, [window] { automaticCheck(window); });
}

QWidget *DoseUpdate::settingsGroup()
{
    auto *group = new QGroupBox(tr("Updates"));
    group->setObjectName(u"doseUpdates"_s);
    auto *column = new QVBoxLayout(group);
    auto *daily = new QCheckBox(tr("Check for tTorrent updates once a day"));
    daily->setObjectName(u"doseUpdateDaily"_s);
    daily->setChecked(settings()->loadValue<bool>(KEY_AUTO, true));
    daily->setToolTip(tr("Asks GitHub for the list of tTorrent releases. Nothing is downloaded without you."));
    auto *previews = new QCheckBox(tr("Include preview releases"));
    previews->setObjectName(u"doseUpdatePreviews"_s);
    previews->setChecked(settings()->loadValue<bool>(KEY_PREVIEWS, true));
    column->addWidget(daily);
    column->addWidget(previews);
    return group;
}

void DoseUpdate::saveSettings(QWidget *group)
{
    if (!group)
        return;
    if (auto *daily = group->findChild<QCheckBox *>(u"doseUpdateDaily"_s))
        settings()->storeValue(KEY_AUTO, daily->isChecked());
    if (auto *previews = group->findChild<QCheckBox *>(u"doseUpdatePreviews"_s))
        settings()->storeValue(KEY_PREVIEWS, previews->isChecked());
}
