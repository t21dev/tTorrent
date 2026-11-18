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
#include <QActionGroup>
#include <QComboBox>
#include <QLabel>
#include <QLineEdit>
#include <QMenu>
#include <QPainter>
#include <QStyle>
#include <QSvgRenderer>
#include <QToolBar>
#include <QToolButton>

#include "base/settingsstorage.h"
#include "gui/uithememanager.h"

namespace
{
    const QString HIDDEN_KEY = QStringLiteral("Appearance/ToolbarHidden");
    const QString ICON_SIZE_KEY = QStringLiteral("Appearance/ToolbarIconSize");

    int storedIconSize()
    {
        const int px = SettingsStorage::instance()->loadValue<int>(ICON_SIZE_KEY, 16);
        return ((px == 16) || (px == 20) || (px == 24)) ? px : 16;
    }

    QStringList hiddenButtons()
    {
        return SettingsStorage::instance()->loadValue<QStringList>(HIDDEN_KEY);
    }

    // A line icon drawn in one colour (the primary button's plus).
    QIcon lineIcon(const QString &paths, const QColor &colour)
    {
        const QString svg = QStringLiteral("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%1' "
            "stroke-width='2.2' stroke-linecap='round' stroke-linejoin='round'>%2</svg>").arg(colour.name(), paths);
        QSvgRenderer renderer {svg.toUtf8()};
        QIcon icon;
        for (const qreal scale : {1.0, 1.25, 1.5, 2.0})
        {
            QPixmap pixmap {QSize(16, 16) * scale};
            pixmap.setDevicePixelRatio(scale);
            pixmap.fill(Qt::transparent);
            QPainter painter {&pixmap};
            renderer.render(&painter, QRectF(0, 0, 16, 16));
            icon.addPixmap(pixmap);
        }
        return icon;
    }

    // "Add torrent" is the one primary button: accent fill, icon and label.
    void stylePrimary(QToolBar *toolBar)
    {
        const UIThemeManager *theme = UIThemeManager::instance();
        if (!theme->usesDoseTheme())
            return;
        auto *open = toolBar->findChild<QAction *>(QStringLiteral("actionOpen"), Qt::FindDirectChildrenOnly);
        if (!open)
            open = toolBar->window()->findChild<QAction *>(QStringLiteral("actionOpen"));
        auto *button = open ? qobject_cast<QToolButton *>(toolBar->widgetForAction(open)) : nullptr;
        if (!button)
            return;
        button->setObjectName(QStringLiteral("dosePrimary"));
        button->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
        open->setIconText(QToolBar::tr("Add torrent")); // the toolbar label; menus keep "Open…"
        button->setIcon(lineIcon(QStringLiteral("<path d='M12 5v14M5 12h14'/>"), theme->doseToken(QStringLiteral("onAccent"))));
        button->style()->unpolish(button);
        button->style()->polish(button);
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
        stylePrimary(toolBar);
    }
}

void DoseToolbar::install(QToolBar *toolBar, QMenu *contextMenu)
{
    if (!toolBar || !contextMenu)
        return;
    const QList<QAction *> original = toolBar->actions();

    toolBar->setIconSize({storedIconSize(), storedIconSize()});

    contextMenu->addSeparator();
    QMenu *sizes = contextMenu->addMenu(QToolBar::tr("Icon size"));
    auto *sizeGroup = new QActionGroup(sizes);
    for (const auto &[px, name] : {std::pair {16, QToolBar::tr("Small")}, std::pair {20, QToolBar::tr("Medium")}, std::pair {24, QToolBar::tr("Large")}})
    {
        QAction *choice = sizes->addAction(name);
        choice->setCheckable(true);
        choice->setChecked(px == storedIconSize());
        sizeGroup->addAction(choice);
        QObject::connect(choice, &QAction::triggered, toolBar, [toolBar, px]
        {
            SettingsStorage::instance()->storeValue(ICON_SIZE_KEY, px);
            toolBar->setIconSize({px, px});
        });
    }
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
    // The theme can change while running: redraw the primary button's icon.
    QObject::connect(UIThemeManager::instance(), &UIThemeManager::themeChanged, toolBar, [toolBar] { stylePrimary(toolBar); });
}

void DoseToolbar::polishFilter(QLineEdit *edit, QComboBox *columns)
{
    if (!edit || !columns || !UIThemeManager::instance()->usesDoseTheme())
        return;
    edit->setFixedWidth(300);
    edit->setPlaceholderText(QToolBar::tr("Filter torrents  (Ctrl+F)"));
    columns->hide();
    if (QWidget *row = columns->parentWidget())
    {
        if (auto *label = row->findChild<QLabel *>(QString(), Qt::FindDirectChildrenOnly))
            label->hide();
    }

    // The search icon at the start of the field opens "Filter by".
    const QList<QAction *> actions = edit->actions();
    QAction *icon = actions.isEmpty() ? nullptr : actions.first();
    if (!icon)
        return;
    auto *menu = new QMenu(edit);
    const auto fill = [menu, columns]
    {
        menu->clear();
        menu->addSection(QToolBar::tr("Filter by"));
        for (int i = 0; i < columns->count(); ++i)
        {
            QAction *choice = menu->addAction(columns->itemText(i));
            choice->setCheckable(true);
            choice->setChecked(i == columns->currentIndex());
            QObject::connect(choice, &QAction::triggered, columns, [columns, i] { columns->setCurrentIndex(i); });
        }
    };
    icon->setToolTip(QToolBar::tr("Filter by: %1").arg(columns->currentText()));
    QObject::connect(columns, &QComboBox::currentIndexChanged, icon, [icon, columns, edit]
    {
        icon->setToolTip(QToolBar::tr("Filter by: %1").arg(columns->currentText()));
        edit->setPlaceholderText(QToolBar::tr("Filter by %1  (Ctrl+F)").arg(columns->currentText().toLower()));
    });
    QObject::connect(icon, &QAction::triggered, edit, [edit, menu, fill]
    {
        fill();
        menu->popup(edit->mapToGlobal(QPoint(0, edit->height() + 4)));
    });
}

int DoseToolbar::iconSize()
{
    return storedIconSize();
}

void DoseToolbar::setIconSize(QToolBar *toolBar, const int px)
{
    SettingsStorage::instance()->storeValue(ICON_SIZE_KEY, px);
    if (toolBar)
        toolBar->setIconSize({px, px});
}
