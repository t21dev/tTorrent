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

#include "dosetrim.h"

#include <QAction>
#include <QComboBox>
#include <QDialog>
#include <QLabel>
#include <QListWidget>
#include <QMainWindow>
#include <QMenu>
#include <QStackedWidget>
#include <QHeaderView>
#include <QTableWidget>

#include "base/bittorrent/session.h"
#include "base/global.h"
#include "base/preferences.h"
#include "gui/uithememanager.h"

namespace
{
    void hideAction(QMainWindow *window, const char *name)
    {
        if (auto *action = window->findChild<QAction *>(QLatin1String(name)))
            action->setVisible(false);
    }

    void hideWidget(QWidget *root, const char *name)
    {
        if (auto *widget = root->findChild<QWidget *>(QLatin1String(name)))
            widget->hide();
    }
}

void DoseTrim::applyToMainWindow(QMainWindow *window)
{
    // Keep the features off even if an imported profile had them on.
    Preferences *pref = Preferences::instance();
    pref->setSearchEnabled(false);
    pref->setUpdateCheckEnabled(false);
    pref->setMailNotificationEnabled(false);
    pref->setAltWebUIEnabled(false);
    pref->resolvePeerHostNames(false);
    BitTorrent::Session *session = BitTorrent::Session::instance();
    session->setTrackerEnabled(false);
    session->setI2PEnabled(false);

    hideAction(window, "actionSearchWidget");
    hideAction(window, "actionDonateMoney");
    hideAction(window, "actionCheckForUpdates");
    hideAction(window, "actionManagePlugins");
    // A hidden QMenu still leaves its title in the menu bar.
    if (auto *plugins = window->findChild<QMenu *>(QStringLiteral("menuPlugins")))
        plugins->menuAction()->setVisible(false);
}

void DoseTrim::applyToOptions(QDialog *dialog)
{
    for (const char *name : {"checkProgramUpdates", "groupMailNotification", "groupI2P", "groupAltWebUI"})
        hideWidget(dialog, name);

    // Tray icon: outline or filled monochrome, besides the colour tile.
    if (auto *tray = dialog->findChild<QComboBox *>(u"comboTrayIcon"_s); tray && (tray->count() == 2))
    {
        tray->setItemText(1, QDialog::tr("Monochrome (outline)"));
        tray->addItem(QDialog::tr("Monochrome (filled)"));
        tray->setCurrentIndex(static_cast<int>(UIThemeManager::instance()->trayIconStyle()));
    }

    // The Search page and its entry in the sections list.
    auto *pages = dialog->findChild<QStackedWidget *>(QStringLiteral("tabOption"));
    auto *sections = dialog->findChild<QListWidget *>(QStringLiteral("tabSelection"));
    if (pages && sections)
    {
        if (QWidget *search = pages->findChild<QWidget *>(QStringLiteral("tabSearchPage")))
        {
            const int index = pages->indexOf(search);
            if ((index >= 0) && (index < sections->count()))
            {
                sections->item(index)->setHidden(true);
                sections->item(index)->setData(Qt::UserRole + 77, true);
            }
        }
    }

    // Advanced: rows for the embedded tracker, host-name lookups and I2P.
    for (QTableWidget *table : dialog->findChildren<QTableWidget *>())
    {
        table->verticalHeader()->setDefaultSectionSize(38);
        table->verticalHeader()->setSectionResizeMode(QHeaderView::Fixed);
        for (int row = 0; row < table->rowCount(); ++row)
        {
            const auto *label = qobject_cast<QLabel *>(table->cellWidget(row, 0));
            const QString text = label ? label->text() : QString();
            // The app's own section is tTorrent's, not qBittorrent's.
            if (label && text.contains(u"qBittorrent Section"))
                const_cast<QLabel *>(label)->setText(QString(text).replace(u"qBittorrent Section"_s, QDialog::tr("tTorrent")));
            if (text.contains(u"embedded tracker", Qt::CaseInsensitive)
                || text.contains(u"Resolve peer host names", Qt::CaseInsensitive)
                || text.startsWith(u"I2P", Qt::CaseInsensitive))
            {
                table->setRowHidden(row, true);
            }
        }
    }
}
