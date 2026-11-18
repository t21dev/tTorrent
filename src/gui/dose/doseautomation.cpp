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

#include "doseautomation.h"
#include "dosebackup.h"
#include "dosemcp.h"
#include "dosememory.h"
#include "dosepair.h"
#include "doseupdate.h"

#include <functional>

#include <QCheckBox>
#include <QCoreApplication>
#include <QRegularExpression>
#include <QDateTime>
#include <QDir>
#include <QFrame>
#include <QGraphicsOpacityEffect>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QMainWindow>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QPointer>
#include <QProcess>
#include <QPropertyAnimation>
#include <QPushButton>
#include <QSpinBox>
#include <QStandardPaths>
#include <QStorageInfo>
#include <QTimer>
#include <QToolButton>
#include <QTreeView>
#include <QUrl>
#include <QVBoxLayout>

#include "base/bittorrent/infohash.h"
#include "base/bittorrent/session.h"
#include "base/bittorrent/torrent.h"
#include "base/global.h"
#include "base/logger.h"
#include "base/path.h"
#include "base/settingsstorage.h"
#include "base/utils/misc.h"
#include "gui/uithememanager.h"
#include "gui/utils.h"

namespace
{
    const QString KEY_GUARD = u"tTorrent/LowDiskGuard"_s;
    const QString KEY_GUARD_GIB = u"tTorrent/LowDiskGiB"_s;
    const QString KEY_GUARD_STOPPED = u"tTorrent/LowDiskStopped"_s;
    const QString KEY_WEBHOOK = u"tTorrent/WebhookUrl"_s;
    const QString KEY_UNPACK = u"tTorrent/UnpackArchives"_s;
    const QString KEY_CARDS = u"tTorrent/NotificationCards"_s;

    SettingsStorage *settings()
    {
        return SettingsStorage::instance();
    }

    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseAutomation", text);
    }

    // ---- Notification cards -------------------------------------------------

    // A card in the bottom-right corner of the main window: what happened,
    // and buttons to act on it. Fades out after a few seconds.
    void showCard(QMainWindow *window, const QString &title, const QString &text, const QString &iconId
        , const QList<std::pair<QString, std::function<void ()>>> &actions, const bool always = false)
    {
        if (!window || !window->isVisible() || window->isMinimized() || (!always && !DoseAutomation::notificationCards()))
            return;
        // One card at a time: a new one replaces the old.
        for (QFrame *old : window->findChildren<QFrame *>(u"doseCard"_s, Qt::FindDirectChildrenOnly))
            old->deleteLater();

        auto *card = new QFrame(window);
        card->setObjectName(u"doseCard"_s);
        auto *row = new QHBoxLayout(card);
        row->setContentsMargins(12, 10, 8, 10);
        row->setSpacing(10);
        auto *icon = new QLabel;
        icon->setPixmap(UIThemeManager::instance()->getIcon(iconId).pixmap(18, 18));
        icon->setAlignment(Qt::AlignTop);
        row->addWidget(icon);
        auto *column = new QVBoxLayout;
        column->setSpacing(2);
        auto *heading = new QLabel(title);
        heading->setObjectName(u"doseCardTitle"_s);
        auto *body = new QLabel(text);
        body->setObjectName(u"doseCardText"_s);
        body->setMaximumWidth(320);
        body->setWordWrap(true);
        // Sentences wrap; a very long torrent name is shortened in the middle.
        body->setText((text.size() > 140) ? body->fontMetrics().elidedText(text, Qt::ElideMiddle, 2 * 320) : text);
        body->setToolTip(text);
        column->addWidget(heading);
        column->addWidget(body);
        auto *buttons = new QHBoxLayout;
        buttons->setSpacing(6);
        buttons->setContentsMargins(0, 6, 0, 0);
        for (const auto &[label, run] : actions)
        {
            auto *button = new QPushButton(label);
            button->setObjectName(u"doseCardButton"_s);
            button->setCursor(Qt::PointingHandCursor);
            QObject::connect(button, &QPushButton::clicked, card, [card, run]
            {
                run();
                card->deleteLater();
            });
            buttons->addWidget(button);
        }
        buttons->addStretch();
        column->addLayout(buttons);
        row->addLayout(column, 1);
        auto *close = new QToolButton;
        close->setObjectName(u"doseCardClose"_s);
        close->setIcon(UIThemeManager::instance()->getIcon(u"edit-clear"_s));
        close->setAutoRaise(true);
        close->setFixedSize(22, 22);
        close->setToolTip(tr("Dismiss"));
        QObject::connect(close, &QToolButton::clicked, card, &QObject::deleteLater);
        row->addWidget(close, 0, Qt::AlignTop);

        card->adjustSize();
        const auto place = [window, card]
        {
            const QRect area = window->centralWidget() ? window->centralWidget()->geometry() : window->rect();
            card->move(area.right() - card->width() - 16, area.bottom() - card->height() - 16);
        };
        place();
        card->show();
        card->raise();

        auto *effect = new QGraphicsOpacityEffect(card);
        card->setGraphicsEffect(effect);
        auto *fadeIn = new QPropertyAnimation(effect, "opacity", card);
        fadeIn->setDuration(160);
        fadeIn->setStartValue(0.0);
        fadeIn->setEndValue(1.0);
        // The effect is only for the fade; drop it after, so text stays crisp.
        QObject::connect(fadeIn, &QPropertyAnimation::finished, card, [card] { card->setGraphicsEffect(nullptr); });
        fadeIn->start(QAbstractAnimation::DeleteWhenStopped);

        // Stays while the pointer is over it.
        auto *timer = new QTimer(card);
        timer->setInterval(8000);
        QObject::connect(timer, &QTimer::timeout, card, [card]
        {
            if (card->underMouse())
                return;
            card->deleteLater();
        });
        timer->start();
    }

    void showTorrent(QMainWindow *window, const QString &name)
    {
        if (window->isMinimized() || !window->isVisible())
            window->showNormal();
        window->raise();
        window->activateWindow();
        for (QTreeView *view : window->findChildren<QTreeView *>())
        {
            if (!view->inherits("TransferListWidget") || !view->model())
                continue;
            const QModelIndexList found = view->model()->match(view->model()->index(0, 0), Qt::DisplayRole, name, 1, Qt::MatchExactly);
            if (!found.isEmpty())
            {
                view->setCurrentIndex(found.first());
                view->scrollTo(found.first());
            }
        }
    }

    // ---- Webhooks -----------------------------------------------------------

    QNetworkAccessManager *network()
    {
        static auto *manager = new QNetworkAccessManager(qApp);
        return manager;
    }

    void callWebhook(const QString &event, const BitTorrent::Torrent *torrent, const QString &detail = {})
    {
        const QUrl url {DoseAutomation::webhookUrl()};
        if (!url.isValid() || !url.scheme().startsWith(u"http"))
            return;
        QJsonObject body {
            {u"event"_s, event},
            {u"time"_s, QDateTime::currentDateTimeUtc().toString(Qt::ISODate)},
            {u"app"_s, u"tTorrent"_s},
        };
        if (torrent)
        {
            body.insert(u"name"_s, torrent->name());
            body.insert(u"hash"_s, torrent->id().toString());
            body.insert(u"size"_s, torrent->wantedSize());
            body.insert(u"savePath"_s, torrent->savePath().toString());
        }
        if (!detail.isEmpty())
            body.insert(u"detail"_s, detail);
        QNetworkRequest request {url};
        request.setHeader(QNetworkRequest::ContentTypeHeader, u"application/json"_s);
        request.setHeader(QNetworkRequest::UserAgentHeader, u"tTorrent"_s);
        request.setTransferTimeout(10000);
        QNetworkReply *reply = network()->post(request, QJsonDocument(body).toJson(QJsonDocument::Compact));
        QObject::connect(reply, &QNetworkReply::finished, reply, [reply, event]
        {
            if (reply->error() != QNetworkReply::NoError)
                LogMsg(tr("Webhook for \"%1\" failed: %2").arg(event, reply->errorString()), Log::WARNING);
            reply->deleteLater();
        });
    }

    // ---- Unpack when finished ----------------------------------------------

    // The first volume of a multi-part RAR is ".part1.rar" (or ".part01.rar");
    // the other parts are opened through it.
    bool isFirstVolume(const QString &fileName)
    {
        static const QRegularExpression part {u"\\.part(\\d+)\\.rar$"_s, QRegularExpression::CaseInsensitiveOption};
        const QRegularExpressionMatch match = part.match(fileName);
        return !match.hasMatch() || (match.captured(1).toInt() == 1);
    }

    void unpack(QMainWindow *window, const BitTorrent::Torrent *torrent)
    {
        if (!DoseAutomation::unpackArchives())
            return;
        const QString tar = QStandardPaths::findExecutable(u"tar"_s, {QDir::fromNativeSeparators(qEnvironmentVariable("SystemRoot")) + u"/System32"_s});
        if (tar.isEmpty())
        {
            LogMsg(tr("Can't unpack archives: Windows' tar.exe was not found."), Log::WARNING);
            return;
        }
        const QString name = torrent->name();
        for (int i = 0; i < torrent->filesCount(); ++i)
        {
            const Path file = torrent->actualFilePath(i);
            const QString ext = file.extension().toLower();
            if (((ext != u".zip") && (ext != u".rar") && (ext != u".7z")) || !isFirstVolume(file.filename()))
                continue;
            const Path archive = torrent->actualStorageLocation() / file;
            Path target = archive;
            target.removeExtension();
            if (target.filename().contains(u".part", Qt::CaseInsensitive))
                target.removeExtension();
            QDir().mkpath(target.data());
            auto *process = new QProcess(qApp);
            process->setProgram(tar);
            process->setArguments({u"-xf"_s, archive.toString(), u"-C"_s, target.toString()});
            QPointer<QMainWindow> guard {window};
            QObject::connect(process, &QProcess::finished, process, [process, guard, archive, target, name](const int code)
            {
                if (code == 0)
                {
                    LogMsg(tr("Unpacked \"%1\" into \"%2\".").arg(archive.toString(), target.toString()));
                    if (guard)
                    {
                        showCard(guard, tr("Unpacked"), archive.filename(), u"task-complete"_s
                            , {{tr("Open folder"), [target] { Utils::Gui::openPath(target); }}});
                    }
                }
                else
                {
                    LogMsg(tr("Couldn't unpack \"%1\": %2").arg(archive.toString(), QString::fromLocal8Bit(process->readAllStandardError()).trimmed()), Log::WARNING);
                }
                process->deleteLater();
            });
            process->start(QIODevice::ReadOnly);
        }
    }

    // ---- Low disk space guard -----------------------------------------------

    class DiskGuard final : public QObject
    {
    public:
        explicit DiskGuard(QMainWindow *window)
            : QObject(window)
            , m_window {window}
        {
            auto *timer = new QTimer(this);
            timer->setInterval(30 * 1000);
            connect(timer, &QTimer::timeout, this, &DiskGuard::check);
            timer->start();
            QTimer::singleShot(10 * 1000, this, &DiskGuard::check);
        }

    private:
        static qint64 freeBytes(const Path &path)
        {
            const QStorageInfo info {path.data()};
            return info.isValid() ? info.bytesAvailable() : -1;
        }

        void check()
        {
            BitTorrent::Session *session = BitTorrent::Session::instance();
            QStringList stopped = settings()->loadValue<QStringList>(KEY_GUARD_STOPPED);
            const qint64 limit = qint64(DoseAutomation::lowDiskGiB()) * 1024 * 1024 * 1024;
            const bool on = DoseAutomation::lowDiskGuard();
            bool changed = false;

            // Start again the ones we stopped, once their drive has room
            // (half as much again, so they don't flip on and off).
            for (const QString &id : QStringList(stopped))
            {
                BitTorrent::Torrent *torrent = session->getTorrent(BitTorrent::TorrentID::fromString(id));
                if (!torrent || !torrent->isStopped())
                {
                    stopped.removeAll(id);
                    changed = true;
                    continue;
                }
                const qint64 free = freeBytes(torrent->savePath());
                if (!on || (free < 0) || (free > (limit + (limit / 2))))
                {
                    torrent->start();
                    stopped.removeAll(id);
                    changed = true;
                    if (on)
                        LogMsg(tr("Space is back on the drive: started \"%1\" again.").arg(torrent->name()));
                }
            }

            if (on)
            {
                int count = 0;
                QString drive;
                QHash<QString, qint64> freeByRoot;
                for (BitTorrent::Torrent *torrent : session->torrents())
                {
                    if (torrent->isFinished() || torrent->isStopped())
                        continue;
                    const QString root = QStorageInfo(torrent->savePath().data()).rootPath();
                    if (!freeByRoot.contains(root))
                        freeByRoot.insert(root, freeBytes(torrent->savePath()));
                    const qint64 free = freeByRoot.value(root);
                    if ((free >= 0) && (free < limit))
                    {
                        torrent->stop();
                        stopped.append(torrent->id().toString());
                        changed = true;
                        ++count;
                        drive = QDir::toNativeSeparators(root);
                        LogMsg(tr("Low disk space on %1: stopped \"%2\".").arg(drive, torrent->name()), Log::WARNING);
                    }
                }
                if (count > 0)
                {
                    const QString text = tr("%1 download(s) stopped: less than %2 GiB free on %3. They start again when there is room.")
                        .arg(QString::number(count), QString::number(DoseAutomation::lowDiskGiB()), drive);
                    showCard(m_window, tr("Low disk space"), text, u"dialog-warning"_s, {});
                }
            }
            if (changed)
                settings()->storeValue(KEY_GUARD_STOPPED, stopped);
        }

        QMainWindow *m_window = nullptr;
    };
}

void DoseAutomation::showCard(QMainWindow *window, const QString &title, const QString &text, const QString &iconId
    , const QList<std::pair<QString, std::function<void ()>>> &actions, const bool always)
{
    ::showCard(window, title, text, iconId, actions, always);
}

bool DoseAutomation::lowDiskGuard()
{
    return settings()->loadValue<bool>(KEY_GUARD, true);
}

int DoseAutomation::lowDiskGiB()
{
    return std::clamp(settings()->loadValue<int>(KEY_GUARD_GIB, 5), 1, 1000);
}

QString DoseAutomation::webhookUrl()
{
    return settings()->loadValue<QString>(KEY_WEBHOOK).trimmed();
}

bool DoseAutomation::unpackArchives()
{
    return settings()->loadValue<bool>(KEY_UNPACK, false);
}

bool DoseAutomation::notificationCards()
{
    return settings()->loadValue<bool>(KEY_CARDS, true);
}

void DoseAutomation::install(QMainWindow *window)
{
    if (!window)
        return;
    BitTorrent::Session *session = BitTorrent::Session::instance();
    new DiskGuard(window);
    DosePair::install(window);
    DoseMcp::apply();
    DoseMemory::install(window);
    DoseUpdate::install(window);
    DoseBackup::install(window);

    QObject::connect(session, &BitTorrent::Session::torrentAdded, window, [](BitTorrent::Torrent *torrent)
    {
        callWebhook(u"added"_s, torrent);
    });
    QObject::connect(session, &BitTorrent::Session::torrentFinished, window, [window](BitTorrent::Torrent *torrent)
    {
        callWebhook(u"finished"_s, torrent);
        const QString name = torrent->name();
        const Path content = torrent->contentPath();
        showCard(window, tr("Download finished"), name, u"checked-completed"_s, {
            {tr("Open folder"), [content] { Utils::Gui::openFolderSelect(content); }},
            {tr("Show"), [window, name] { showTorrent(window, name); }},
        });
        unpack(window, torrent);
    });
    QObject::connect(session, &BitTorrent::Session::torrentIOError, window, [window](BitTorrent::Torrent *torrent, const QString &message)
    {
        callWebhook(u"failed"_s, torrent, message);
        const QString name = torrent->name();
        showCard(window, tr("Download failed"), name + u" · "_s + message, u"error"_s, {
            {tr("Show"), [window, name] { showTorrent(window, name); }},
        });
    });
    QObject::connect(session, &BitTorrent::Session::addTorrentFailed, window, [](const BitTorrent::InfoHash &, const QString &reason)
    {
        callWebhook(u"failed"_s, nullptr, reason);
    });
}

QWidget *DoseAutomation::settingsGroup()
{
    auto *group = new QGroupBox(tr("When something happens"));
    group->setObjectName(u"doseAutomation"_s);
    auto *grid = new QGridLayout(group);
    int row = 0;

    auto *guard = new QCheckBox(tr("Stop downloads below"));
    guard->setToolTip(tr("Downloads on a drive stop when its free space drops below this, and start again once there is room."));
    guard->setObjectName(u"doseGuard"_s);
    guard->setChecked(lowDiskGuard());
    auto *gib = new QSpinBox;
    gib->setObjectName(u"doseGuardGiB"_s);
    gib->setRange(1, 1000);
    gib->setSuffix(u" GiB"_s);
    gib->setValue(lowDiskGiB());
    gib->setEnabled(guard->isChecked());
    QObject::connect(guard, &QCheckBox::toggled, gib, &QWidget::setEnabled);
    auto *guardRow = new QHBoxLayout;
    guardRow->addWidget(guard);
    guardRow->addWidget(gib);
    guardRow->addWidget(new QLabel(tr("free on the drive")));
    guardRow->addStretch();
    grid->addLayout(guardRow, row++, 0, 1, 2);

    auto *unpackBox = new QCheckBox(tr("Unpack .zip, .rar and .7z files when finished"));
    unpackBox->setObjectName(u"doseUnpack"_s);
    unpackBox->setChecked(unpackArchives());
    grid->addWidget(unpackBox, row++, 0, 1, 2);

    auto *cards = new QCheckBox(tr("Show a card when a torrent finishes or fails"));
    cards->setToolTip(tr("A card in the window's corner with \"Open folder\" and \"Show\"."));
    cards->setObjectName(u"doseCards"_s);
    cards->setChecked(notificationCards());
    grid->addWidget(cards, row++, 0, 1, 2);

    grid->addWidget(new QLabel(tr("Webhook URL:")), row, 0);
    auto *hookRow = new QHBoxLayout;
    auto *hook = new QLineEdit(webhookUrl());
    hook->setObjectName(u"doseWebhook"_s);
    hook->setPlaceholderText(u"https://example.com/hooks/torrents"_s);
    hook->setToolTip(tr("Called with a JSON POST when a torrent is added, finishes or fails. Leave empty to turn off."));
    auto *test = new QPushButton(tr("Send test"));
    QObject::connect(test, &QPushButton::clicked, hook, [hook, test]
    {
        const QUrl url {hook->text().trimmed()};
        if (!url.isValid() || !url.scheme().startsWith(u"http"))
        {
            test->setText(tr("Not a web address"));
            return;
        }
        QNetworkRequest request {url};
        request.setHeader(QNetworkRequest::ContentTypeHeader, u"application/json"_s);
        request.setTransferTimeout(10000);
        const QJsonObject body {{u"event"_s, u"test"_s}, {u"app"_s, u"tTorrent"_s}, {u"time"_s, QDateTime::currentDateTimeUtc().toString(Qt::ISODate)}};
        test->setEnabled(false);
        test->setText(tr("Sending…"));
        QNetworkReply *reply = network()->post(request, QJsonDocument(body).toJson(QJsonDocument::Compact));
        QPointer<QPushButton> button {test};
        QObject::connect(reply, &QNetworkReply::finished, reply, [reply, button]
        {
            if (button)
            {
                button->setEnabled(true);
                button->setText((reply->error() == QNetworkReply::NoError) ? tr("Sent") : tr("Failed: %1").arg(reply->errorString()));
            }
            reply->deleteLater();
        });
    });
    hookRow->addWidget(hook, 1);
    hookRow->addWidget(test);
    grid->addLayout(hookRow, row++, 1);
    grid->setColumnStretch(1, 1);
    return group;
}

void DoseAutomation::saveSettings(QWidget *group)
{
    if (!group)
        return;
    if (auto *guard = group->findChild<QCheckBox *>(u"doseGuard"_s))
        settings()->storeValue(KEY_GUARD, guard->isChecked());
    if (auto *gib = group->findChild<QSpinBox *>(u"doseGuardGiB"_s))
        settings()->storeValue(KEY_GUARD_GIB, gib->value());
    if (auto *unpackBox = group->findChild<QCheckBox *>(u"doseUnpack"_s))
        settings()->storeValue(KEY_UNPACK, unpackBox->isChecked());
    if (auto *cards = group->findChild<QCheckBox *>(u"doseCards"_s))
        settings()->storeValue(KEY_CARDS, cards->isChecked());
    if (auto *hook = group->findChild<QLineEdit *>(u"doseWebhook"_s))
        settings()->storeValue(KEY_WEBHOOK, hook->text().trimmed());
}
