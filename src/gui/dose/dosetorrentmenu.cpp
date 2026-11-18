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

#include "dosetorrentmenu.h"

#include <QAction>
#include <QMenu>

#include "base/global.h"
#include "base/settingsstorage.h"
#include "gui/uithememanager.h"
#include "dosepaint.h"

namespace
{
    const QString HIDDEN_KEY = u"Appearance/TorrentMenuHidden"_s;

    // Entries a person can hide, in menu order, with their labels.
    QList<std::pair<QString, QString>> entries()
    {
        return {
            {u"start"_s, QMenu::tr("Start")}, {u"stop"_s, QMenu::tr("Stop")}, {u"forceStart"_s, QMenu::tr("Force start")},
            {u"remove"_s, QMenu::tr("Remove")}, {u"openFolder"_s, QMenu::tr("Open destination folder")},
            {u"category"_s, QMenu::tr("Category")}, {u"tags"_s, QMenu::tr("Tags")},
            {u"files"_s, QMenu::tr("Files")}, {u"download"_s, QMenu::tr("Download")}, {u"trackers"_s, QMenu::tr("Trackers")},
            {u"queue"_s, QMenu::tr("Queue")}, {u"copy"_s, QMenu::tr("Copy")},
        };
    }

    void separator(QMenu *menu)
    {
        const QList<QAction *> actions = menu->actions();
        if (!actions.isEmpty() && !actions.last()->isSeparator())
            menu->addSeparator();
    }
}

void DoseTorrentMenu::shape(QMenu *menu, const QHash<QString, QAction *> &named, QMenu *category, QMenu *tags, QMenu *copy)
{
    if (!menu || !DosePaint::active())
        return;

    const QList<QAction *> present = menu->actions();
    QMenu *queue = nullptr;
    for (QAction *a : present)
    {
        if (QMenu *m = a->menu(); m && (m != category) && (m != tags) && (m != copy))
            queue = m;
    }
    for (QAction *a : present)
        menu->removeAction(a);

    const QStringList hidden = SettingsStorage::instance()->loadValue<QStringList>(HIDDEN_KEY);
    const auto shown = [&](const QString &id) { return !hidden.contains(id); };
    const auto has = [&](const QString &id)
    {
        QAction *a = named.value(id);
        return a && present.contains(a);
    };
    const auto add = [&](QMenu *target, const QString &id)
    {
        if (has(id))
            target->addAction(named.value(id));
    };
    const auto group = [&](const QString &id, const QString &title, const QString &icon, const QStringList &ids)
    {
        if (!shown(id))
            return;
        QStringList available;
        for (const QString &i : ids)
        {
            if (has(i))
                available << i;
        }
        if (available.isEmpty())
            return;
        QMenu *sub = menu->addMenu(UIThemeManager::instance()->getIcon(icon), title);
        for (const QString &i : available)
            add(sub, i);
    };

    for (const QString &id : {u"start"_s, u"stop"_s, u"forceStart"_s})
    {
        if (shown(id))
            add(menu, id);
    }
    separator(menu);
    if (shown(u"remove"_s))
        add(menu, u"remove"_s);
    separator(menu);
    if (shown(u"openFolder"_s))
        add(menu, u"openFolder"_s);
    if (category && shown(u"category"_s))
        menu->addMenu(category);
    if (tags && shown(u"tags"_s))
        menu->addMenu(tags);
    separator(menu);

    group(u"files"_s, QMenu::tr("Files"), u"folder-documents"_s
        , {u"setLocation"_s, u"relocate"_s, u"rename"_s, u"manageContent"_s, u"preview"_s, u"recheck"_s, u"export"_s});
    group(u"download"_s, QMenu::tr("Download"), u"go-down"_s
        , {u"options"_s, u"sequential"_s, u"firstLast"_s, u"autoTMM"_s, u"superSeeding"_s});
    group(u"trackers"_s, QMenu::tr("Trackers"), u"trackers"_s, {u"editTrackers"_s, u"reannounce"_s});
    if (queue && shown(u"queue"_s))
        menu->addMenu(queue);
    if (copy && shown(u"copy"_s))
        menu->addMenu(copy);
    separator(menu);

    // Customize: tick what shows; applies the next time the menu opens.
    QMenu *customize = menu->addMenu(UIThemeManager::instance()->getIcon(u"configure"_s), QMenu::tr("Customize this menu"));
    for (const auto &[id, label] : entries())
    {
        QAction *toggle = customize->addAction(label);
        toggle->setCheckable(true);
        toggle->setChecked(shown(id));
        QObject::connect(toggle, &QAction::toggled, customize, [id](const bool on)
        {
            QStringList list = SettingsStorage::instance()->loadValue<QStringList>(HIDDEN_KEY);
            list.removeAll(id);
            if (!on)
                list.append(id);
            SettingsStorage::instance()->storeValue(HIDDEN_KEY, list);
        });
    }
    customize->addSeparator();
    QAction *reset = customize->addAction(QMenu::tr("Show everything"));
    QObject::connect(reset, &QAction::triggered, customize, [] { SettingsStorage::instance()->storeValue(HIDDEN_KEY, QStringList()); });
}
