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

#include "dosetoolbar.h"

#include <QAction>
#include <QMenu>
#include <QToolBar>

#include "base/settingsstorage.h"

namespace
{
    const QString HIDDEN_KEY = QStringLiteral("Appearance/ToolbarHidden");

    QStringList hiddenButtons()
    {
        return SettingsStorage::instance()->loadValue<QStringList>(HIDDEN_KEY);
    }

    // Puts the toolbar back together from `original`, without hidden buttons.
    void rebuild(QToolBar *toolBar, const QList<QAction *> &original)
    {
        const QStringList hidden = hiddenButtons();
        for (QAction *a : original)
            toolBar->removeAction(a);
        // Anything added after install() (search field, spacer) stays last.
        QAction *before = toolBar->actions().value(0, nullptr);
        bool lastWasSeparator = true;
        QList<QAction *> keep;
        for (QAction *a : original)
        {
            if (a->isSeparator())
            {
                if (!lastWasSeparator)
                    keep.append(a);
                lastWasSeparator = true;
                continue;
            }
            if (hidden.contains(a->objectName()))
                continue;
            keep.append(a);
            lastWasSeparator = false;
        }
        while (!keep.isEmpty() && keep.last()->isSeparator())
            keep.removeLast();
        for (QAction *a : keep)
            toolBar->insertAction(before, a);
    }
}

void DoseToolbar::install(QToolBar *toolBar, QMenu *contextMenu)
{
    if (!toolBar || !contextMenu)
        return;
    const QList<QAction *> original = toolBar->actions();

    contextMenu->addSeparator();
    QMenu *show = contextMenu->addMenu(QToolBar::tr("Show buttons"));
    for (QAction *a : original)
    {
        if (a->isSeparator() || a->objectName().isEmpty())
            continue;
        QString label = a->text();
        label.remove(QLatin1Char('&'));
        QAction *toggle = show->addAction(a->icon(), label);
        toggle->setCheckable(true);
        toggle->setChecked(!hiddenButtons().contains(a->objectName()));
        const QString name = a->objectName();
        QObject::connect(toggle, &QAction::toggled, toolBar, [toolBar, original, name](const bool on)
        {
            QStringList hidden = hiddenButtons();
            hidden.removeAll(name);
            if (!on)
                hidden.append(name);
            SettingsStorage::instance()->storeValue(HIDDEN_KEY, hidden);
            rebuild(toolBar, original);
        });
    }
    contextMenu->addAction(QToolBar::tr("Reset toolbar"), toolBar, [toolBar, original, show]
    {
        SettingsStorage::instance()->storeValue(HIDDEN_KEY, QStringList());
        for (QAction *t : show->actions())
        {
            const QSignalBlocker blocker {t};
            t->setChecked(true);
        }
        rebuild(toolBar, original);
    });

    rebuild(toolBar, original);
}
