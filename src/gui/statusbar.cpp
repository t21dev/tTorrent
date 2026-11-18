/*
 * Bittorrent Client using Qt and libtorrent.
 * Copyright (C) 2006  Christophe Dumez <chris@qbittorrent.org>
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
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301, USA.
 *
 * In addition, as a special exception, the copyright holders give permission to
 * link this program with the OpenSSL project's "OpenSSL" library (or with
 * modified versions of it that use the same license as the "OpenSSL" library),
 * and distribute the linked executables. You must obey the GNU General Public
 * License in all respects for all of the code used other than "OpenSSL".  If you
 * modify file(s), you may extend this exception to your version of the file(s),
 * but you are not obligated to do so. If you do not wish to do so, delete this
 * exception statement from your version.
 */

#include "statusbar.h"

#include <utility>

#include <QContextMenuEvent>
#include <QFrame>
#include <QLabel>
#include <QMenu>
#include <QPushButton>
#include <QStyle>

#include "base/bittorrent/session.h"
#include "base/bittorrent/sessionstatus.h"
#include "base/preferences.h"
#include "base/settingsstorage.h"
#include "dose/dosememory.h"
#include "dose/dosestatus.h"
#include "dose/dosepaint.h"
#include "base/utils/misc.h"
#include "speedlimitdialog.h"
#include "uithememanager.h"
#include "utils.h"

#ifdef Q_OS_WIN
#include <windows.h>
#include <psapi.h>
#endif

namespace
{
    // Items the user hid (Appearance/StatusBarHidden). Free space and the
    // external IP keep qBittorrent's own preferences instead.
    const QString HIDDEN_KEY = u"Appearance/StatusBarHidden"_s;

    // This process's private working set: what Task Manager shows as Memory.
    qint64 processMemory()
    {
#ifdef Q_OS_WIN
        // PROCESS_MEMORY_COUNTERS_EX2's layout (Windows 10 1809+): the SDK only
        // declares it for newer target versions than this build sets.
        struct
        {
            PROCESS_MEMORY_COUNTERS_EX ex;
            SIZE_T privateWorkingSetSize;
            SIZE_T sharedCommitUsage;
        } counters {};
        counters.ex.cb = sizeof(counters);
        if (::GetProcessMemoryInfo(::GetCurrentProcess(), reinterpret_cast<PROCESS_MEMORY_COUNTERS *>(&counters), counters.ex.cb))
            return static_cast<qint64>(counters.privateWorkingSetSize);
#endif
        return -1;
    }

    QWidget *createSeparator(QWidget *parent)
    {
        auto *separator = new QFrame(parent);
        separator->setFrameStyle(QFrame::VLine);
#ifndef Q_OS_MACOS
        separator->setFrameShadow(QFrame::Raised);
#endif
        return separator;
    }

    QPushButton *createSpeedButton(const QIcon &icon, QWidget *parent)
    {
        auto *button = new QPushButton(icon, {}, parent);
        button->setCursor(Qt::PointingHandCursor);
        button->setFlat(true);
        button->setFocusPolicy(Qt::NoFocus);
        button->setMinimumWidth(200);
        button->setStyleSheet(u"text-align: left;"_s);
        return button;
    }
}

StatusBar::StatusBar(QWidget *parent)
    : QStatusBar(parent)
{
    QString styleSheet = u"QWidget { margin: 0; }"_s;
#ifndef Q_OS_MACOS
    // Redefining global stylesheet breaks certain elements on mac like tabs.
    // Qt checks whether the stylesheet class inherits("QMacStyle") and this becomes false.
    styleSheet += u"QStatusBar::item { border-width: 0; }";
#endif
    setStyleSheet(styleSheet);

    const auto *session = BitTorrent::Session::instance();
    connect(session, &BitTorrent::Session::speedLimitModeChanged, this, &StatusBar::updateAltSpeedsBtn);

    m_connecStatusLblIcon = new QPushButton(UIThemeManager::instance()->getIcon(u"firewalled"_s), {}, this);
    m_connecStatusLblIcon->setCursor(Qt::PointingHandCursor);
    m_connecStatusLblIcon->setFlat(true);
    m_connecStatusLblIcon->setFocusPolicy(Qt::NoFocus);
    m_connecStatusLblIcon->setToolTip(u"<b>%1</b><br><i>%2</i>"_s.arg(tr("Connection status:")
        , tr("No direct connections. This may indicate network configuration problems.")));
    connect(m_connecStatusLblIcon, &QAbstractButton::clicked, this, &StatusBar::connectionButtonClicked);

    m_dlSpeedLbl = createSpeedButton(UIThemeManager::instance()->getIcon(u"downloading"_s, u"downloading_small"_s), this);
    connect(m_dlSpeedLbl, &QAbstractButton::clicked, this, &StatusBar::capSpeed);

    m_upSpeedLbl = createSpeedButton(UIThemeManager::instance()->getIcon(u"upload"_s, u"seeding"_s), this);
    connect(m_upSpeedLbl, &QAbstractButton::clicked, this, &StatusBar::capSpeed);

    m_freeDiskSpaceLbl = new QLabel(tr("Free space: N/A"));
    m_freeDiskSpaceLbl->setSizePolicy(QSizePolicy::Maximum, QSizePolicy::Preferred);
    DoseStatus::decorate(m_freeDiskSpaceLbl, u"dose-drive"_s); // tTorrent: icon instead of words
    m_freeDiskSpaceSeparator = createSeparator(this);

    m_lastExternalIPsLbl = new QLabel(tr("External IP: N/A"));
    m_lastExternalIPsLbl->setSizePolicy(QSizePolicy::Maximum, QSizePolicy::Preferred);
    m_lastExternalIPsSeparator = createSeparator(this);

    const bool isDHTVisible = session->isDHTEnabled();
    m_DHTLbl = new QLabel(tr("DHT: %1 nodes").arg(0), this);
    m_DHTLbl->setSizePolicy(QSizePolicy::Maximum, QSizePolicy::Preferred);
    DoseStatus::decorate(m_DHTLbl, u"network-server"_s); // tTorrent: icon instead of words
    m_DHTLbl->setVisible(isDHTVisible);
    m_DHTSeparator = createSeparator(this);
    m_DHTSeparator->setVisible(isDHTVisible);

    m_altSpeedsBtn = new QPushButton(this);
    m_altSpeedsBtn->setCursor(Qt::PointingHandCursor);
    m_altSpeedsBtn->setFlat(true);
    m_altSpeedsBtn->setFocusPolicy(Qt::NoFocus);
    updateAltSpeedsBtn(session->isAltGlobalSpeedLimitEnabled());
    connect(m_altSpeedsBtn, &QAbstractButton::clicked, this, &StatusBar::alternativeSpeedsButtonClicked);

    // Because on some platforms the default icon size is bigger
    // and it will result in taller/fatter statusbar, even if the
    // icons are actually 16x16
    const QSize smallIconSize = Utils::Gui::smallIconSize();
    const QSize midIconSize = Utils::Gui::mediumIconSize();
    m_connecStatusLblIcon->setIconSize(smallIconSize);
    m_dlSpeedLbl->setIconSize(smallIconSize);
    m_upSpeedLbl->setIconSize(smallIconSize);
    m_altSpeedsBtn->setIconSize({midIconSize.width(), smallIconSize.height()});

    m_memoryLbl = new QLabel(this);
    m_memoryLbl->setSizePolicy(QSizePolicy::Maximum, QSizePolicy::Preferred);
    m_memoryLbl->setToolTip(tr("Memory tTorrent uses (Task Manager's private working set)"));
    DoseMemory::attachToLabel(m_memoryLbl); // tTorrent: click for the memory graph
    DoseStatus::decorate(m_memoryLbl, u"dose-memory"_s); // tTorrent: icon instead of words
    m_memorySeparator = createSeparator(this);
    m_connectionSeparator = createSeparator(this);
    m_altSpeedsSeparator = createSeparator(this);
    m_dlSpeedSeparator = createSeparator(this);

    // tTorrent: information on the left, connection and speeds on the right.
    addWidget(m_freeDiskSpaceLbl);
    addWidget(m_freeDiskSpaceSeparator);
    addWidget(m_lastExternalIPsLbl);
    addWidget(m_lastExternalIPsSeparator);
    addWidget(m_DHTLbl);
    addWidget(m_DHTSeparator);
    addWidget(m_memoryLbl);
    addWidget(m_memorySeparator);
    addPermanentWidget(m_connecStatusLblIcon);
    addPermanentWidget(m_connectionSeparator);
    addPermanentWidget(m_altSpeedsBtn);
    addPermanentWidget(m_altSpeedsSeparator);
    addPermanentWidget(m_dlSpeedLbl);
    addPermanentWidget(m_dlSpeedSeparator);
    addPermanentWidget(m_upSpeedLbl);
    applyItemVisibility();

    updateExternalAddressesVisibility();

    refresh();
    connect(session, &BitTorrent::Session::statsUpdated, this, &StatusBar::refresh);

    updateFreeDiskSpaceVisibility();
    updateFreeDiskSpaceLabel(session->freeDiskSpace());
    connect(session, &BitTorrent::Session::freeDiskSpaceChecked, this, &StatusBar::updateFreeDiskSpaceLabel);

    connect(Preferences::instance(), &Preferences::changed, this, &StatusBar::optionsSaved);
}

void StatusBar::showRestartRequired()
{
    // Restart required notification
    const QString restartText = tr("qBittorrent needs to be restarted!");

    const QPixmap pixmap = style()->standardIcon(QStyle::SP_MessageBoxWarning).pixmap(Utils::Gui::smallIconSize());
    auto *restartIconLbl = new QLabel(this);
    restartIconLbl->setPixmap(pixmap);
    restartIconLbl->setToolTip(restartText);
    insertWidget(0, restartIconLbl);

    auto *restartLbl = new QLabel(this);
    restartLbl->setText(restartText);
    insertWidget(1, restartLbl);
}

void StatusBar::updateConnectionStatus()
{
    const BitTorrent::SessionStatus &sessionStatus = BitTorrent::Session::instance()->status();

    if (!BitTorrent::Session::instance()->isListening())
    {
        m_connecStatusLblIcon->setIcon(UIThemeManager::instance()->getIcon(u"disconnected"_s));
        m_connecStatusLblIcon->setText(DoseStatus::iconsOnly() ? QString() : tr("Offline")); // tTorrent: a word beside the icon
        const QString tooltip = u"<b>%1</b><br>%2"_s.arg(tr("Connection Status:"), tr("Offline. This usually means that qBittorrent failed to listen on the selected port for incoming connections."));
        m_connecStatusLblIcon->setToolTip(tooltip);
    }
    else
    {
        if (sessionStatus.hasIncomingConnections)
        {
            // Connection OK
            m_connecStatusLblIcon->setIcon(UIThemeManager::instance()->getIcon(u"connected"_s));
            m_connecStatusLblIcon->setText(DoseStatus::iconsOnly() ? QString() : tr("Online")); // tTorrent
            const QString tooltip = u"<b>%1</b><br>%2"_s.arg(tr("Connection Status:"), tr("Online"));
            m_connecStatusLblIcon->setToolTip(tooltip);
        }
        else
        {
            m_connecStatusLblIcon->setIcon(UIThemeManager::instance()->getIcon(u"firewalled"_s));
            m_connecStatusLblIcon->setText(DoseStatus::iconsOnly() ? QString() : tr("Firewalled")); // tTorrent
            const QString tooltip = u"<b>%1</b><br><i>%2</i>"_s.arg(tr("Connection Status:"), tr("No direct connections. This may indicate network configuration problems."));
            m_connecStatusLblIcon->setToolTip(tooltip);
        }
    }
}

void StatusBar::updateDHTNodesNumber()
{
    if (BitTorrent::Session::instance()->isDHTEnabled() && wants(u"dht"_s))
    {
        m_DHTLbl->setVisible(true);
        m_DHTSeparator->setVisible(true);
        const qint64 nodes = BitTorrent::Session::instance()->status().dhtNodes;
        m_DHTLbl->setText(DoseStatus::iconsOnly() ? QString::number(nodes) : tr("DHT: %1 nodes").arg(nodes)); // tTorrent
        m_DHTLbl->setToolTip(DoseStatus::iconsOnly() ? tr("DHT: %1 nodes").arg(nodes) : QString()); // tTorrent
    }
    else
    {
        m_DHTLbl->setVisible(false);
        m_DHTSeparator->setVisible(false);
    }
}

void StatusBar::updateFreeDiskSpaceLabel(const qint64 value)
{
    m_freeDiskSpaceLbl->setText(DoseStatus::text(tr("Free space: "), Utils::Misc::friendlyUnit(value))); // tTorrent
    m_freeDiskSpaceLbl->setToolTip(DoseStatus::iconsOnly() ? tr("Free space") : QString()); // tTorrent
}

void StatusBar::updateFreeDiskSpaceVisibility()
{
    const bool isVisible = Preferences::instance()->isStatusbarFreeDiskSpaceDisplayed();
    m_freeDiskSpaceLbl->setVisible(isVisible);
    m_freeDiskSpaceSeparator->setVisible(isVisible);
}

void StatusBar::updateExternalAddressesLabel()
{
    const QString lastExternalIPv4Address = BitTorrent::Session::instance()->lastExternalIPv4Address();
    const QString lastExternalIPv6Address = BitTorrent::Session::instance()->lastExternalIPv6Address();
    QString addressText = tr("External IP: N/A");

    const bool hasIPv4Address = !lastExternalIPv4Address.isEmpty();
    const bool hasIPv6Address = !lastExternalIPv6Address.isEmpty();

    if (hasIPv4Address && hasIPv6Address)
        addressText = tr("External IPs: %1, %2").arg(lastExternalIPv4Address, lastExternalIPv6Address);
    else if (hasIPv4Address || hasIPv6Address)
        addressText = tr("External IP: %1%2").arg(lastExternalIPv4Address, lastExternalIPv6Address);

    m_lastExternalIPsLbl->setText(addressText);
}

void StatusBar::updateExternalAddressesVisibility()
{
    const bool isVisible = Preferences::instance()->isStatusbarExternalIPDisplayed();
    m_lastExternalIPsLbl->setVisible(isVisible);
    m_lastExternalIPsSeparator->setVisible(isVisible);
}

void StatusBar::updateSpeedLabels()
{
    const BitTorrent::SessionStatus &sessionStatus = BitTorrent::Session::instance()->status();

    QString dlSpeedLbl = Utils::Misc::friendlyUnit(sessionStatus.payloadDownloadRate, true);
    const int dlSpeedLimit = BitTorrent::Session::instance()->downloadSpeedLimit();
    if (dlSpeedLimit > 0)
        dlSpeedLbl += u" [" + Utils::Misc::friendlyUnit(dlSpeedLimit, true) + u']';
    dlSpeedLbl += u" (" + Utils::Misc::friendlyUnit(sessionStatus.totalPayloadDownload) + u')';
    m_dlSpeedLbl->setText(dlSpeedLbl);

    QString upSpeedLbl = Utils::Misc::friendlyUnit(sessionStatus.payloadUploadRate, true);
    const int upSpeedLimit = BitTorrent::Session::instance()->uploadSpeedLimit();
    if (upSpeedLimit > 0)
        upSpeedLbl += u" [" + Utils::Misc::friendlyUnit(upSpeedLimit, true) + u']';
    upSpeedLbl += u" (" + Utils::Misc::friendlyUnit(sessionStatus.totalPayloadUpload) + u')';
    m_upSpeedLbl->setText(upSpeedLbl);
}

void StatusBar::refresh()
{
    updateMemoryLabel();
    updateConnectionStatus();
    updateDHTNodesNumber();
    updateExternalAddressesLabel();
    updateSpeedLabels();
}

void StatusBar::updateAltSpeedsBtn(const bool alternative)
{
    if (alternative)
    {
        m_altSpeedsBtn->setIcon(UIThemeManager::instance()->getIcon(u"slow"_s));
        m_altSpeedsBtn->setToolTip(tr("Click to switch to regular speed limits"));
        m_altSpeedsBtn->setDown(true);
    }
    else
    {
        m_altSpeedsBtn->setIcon(UIThemeManager::instance()->getIcon(u"slow_off"_s));
        m_altSpeedsBtn->setToolTip(tr("Click to switch to alternative speed limits"));
        m_altSpeedsBtn->setDown(false);
    }
    refresh();
}

void StatusBar::capSpeed()
{
    auto *dialog = new SpeedLimitDialog {parentWidget()};
    dialog->setAttribute(Qt::WA_DeleteOnClose);
    dialog->open();
}

void StatusBar::optionsSaved()
{
    updateFreeDiskSpaceVisibility();
    updateExternalAddressesVisibility();
}

bool StatusBar::wants(const QString &item) const
{
    return !SettingsStorage::instance()->loadValue<QStringList>(HIDDEN_KEY).contains(item);
}

void StatusBar::setWanted(const QString &item, const bool on)
{
    QStringList hidden = SettingsStorage::instance()->loadValue<QStringList>(HIDDEN_KEY);
    hidden.removeAll(item);
    if (!on)
        hidden.append(item);
    SettingsStorage::instance()->storeValue(HIDDEN_KEY, hidden);
    applyItemVisibility();
}

void StatusBar::applyItemVisibility()
{
    if (DosePaint::active())
    {
        // Speeds in the number font; down in the accent, up in `ok`.
        for (QWidget *w : {static_cast<QWidget *>(m_dlSpeedLbl), static_cast<QWidget *>(m_upSpeedLbl)})
            w->setProperty("doseNumber", true); // tTorrent: the number face via the stylesheet
        m_dlSpeedLbl->setObjectName(u"doseDown"_s);
        m_upSpeedLbl->setObjectName(u"doseUp"_s);
    }
    const bool memory = wants(u"memory"_s) && (processMemory() >= 0);
    m_memoryLbl->setVisible(memory);
    m_memorySeparator->setVisible(memory);
    m_connecStatusLblIcon->setVisible(wants(u"connection"_s));
    m_connectionSeparator->setVisible(wants(u"connection"_s));
    m_altSpeedsBtn->setVisible(wants(u"altSpeeds"_s));
    m_altSpeedsSeparator->setVisible(wants(u"altSpeeds"_s));
    m_dlSpeedLbl->setVisible(wants(u"download"_s));
    m_dlSpeedSeparator->setVisible(wants(u"download"_s) && wants(u"upload"_s));
    m_upSpeedLbl->setVisible(wants(u"upload"_s));
    updateDHTNodesNumber();
    updateMemoryLabel();
}

void StatusBar::updateMemoryLabel()
{
    if (!m_memoryLbl || !m_memoryLbl->isVisible())
        return;
    const qint64 bytes = processMemory();
    m_memoryLbl->setText(DoseStatus::iconsOnly() ? Utils::Misc::friendlyUnit(bytes) : tr("Memory: %1").arg(Utils::Misc::friendlyUnit(bytes))); // tTorrent
}

void StatusBar::contextMenuEvent(QContextMenuEvent *event)
{
    QMenu menu {this};
    menu.setToolTipsVisible(true);
    const auto add = [&menu](const QString &label, const bool checked, auto onToggle)
    {
        QAction *action = menu.addAction(label);
        action->setCheckable(true);
        action->setChecked(checked);
        QObject::connect(action, &QAction::toggled, &menu, onToggle);
    };
    Preferences *pref = Preferences::instance();
    add(tr("Free space"), pref->isStatusbarFreeDiskSpaceDisplayed(), [pref](const bool on) { pref->setStatusbarFreeDiskSpaceDisplayed(on); pref->apply(); });
    add(tr("External IP"), pref->isStatusbarExternalIPDisplayed(), [pref](const bool on) { pref->setStatusbarExternalIPDisplayed(on); pref->apply(); });
    for (const auto &[id, label] : {std::pair {u"dht"_s, tr("DHT nodes")}, std::pair {u"memory"_s, tr("Memory")}
            , std::pair {u"connection"_s, tr("Connection status")}, std::pair {u"altSpeeds"_s, tr("Alternative speed limits")}
            , std::pair {u"download"_s, tr("Download speed")}, std::pair {u"upload"_s, tr("Upload speed")}})
    {
        add(label, wants(id), [this, id](const bool on) { setWanted(id, on); });
    }
    menu.addSeparator();
    // tTorrent: an icon and a value instead of "Free space: …"
    add(tr("Icons instead of words"), DoseStatus::iconsOnly(), [this](const bool on)
    {
        DoseStatus::setIconsOnly(on);
        DoseStatus::relabel(m_freeDiskSpaceLbl, tr("Free space: "), tr("Free space"));
        DoseStatus::relabel(m_DHTLbl, {}, {});
        DoseStatus::relabel(m_memoryLbl, {}, {});
        refresh();
    });
    menu.addAction(tr("Show all"), this, [this, pref]
    {
        SettingsStorage::instance()->storeValue(HIDDEN_KEY, QStringList());
        pref->setStatusbarFreeDiskSpaceDisplayed(true);
        pref->setStatusbarExternalIPDisplayed(true);
        pref->apply();
        applyItemVisibility();
    });
    menu.exec(event->globalPos());
}
