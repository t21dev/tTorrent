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
#include <QTabBar>
#include <QTabWidget>
#include <QWidgetAction>
#include <QMainWindow>
#include <QHash>
#include <QHBoxLayout>
#include <QFrame>
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
#include "gui/titlebar/dosetitlebar.h"
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

    const QString SLIM_KEY = QStringLiteral("Appearance/ToolbarSlim");

    void applySlim(QToolBar *toolBar, const bool slim)
    {
        if (!toolBar)
            return;
        toolBar->setProperty("doseSlim", slim);
        for (QWidget *w : toolBar->findChildren<QWidget *>())
        {
            w->style()->unpolish(w);
            w->style()->polish(w);
        }
        toolBar->style()->unpolish(toolBar);
        toolBar->style()->polish(toolBar);
        toolBar->updateGeometry();
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
    // A widget of ours that triggers qBittorrent's Open action, so the icon
    // and label stay ours whatever the action does with its own icon.
    QHash<QToolBar *, QWidgetAction *> &primaries()
    {
        static QHash<QToolBar *, QWidgetAction *> map;
        return map;
    }

    QWidgetAction *primaryFor(QToolBar *toolBar, QAction *open)
    {
        if (QWidgetAction *existing = primaries().value(toolBar))
            return existing;
        auto *button = new QToolButton;
        button->setObjectName(QStringLiteral("dosePrimary"));
        button->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
        button->setText(QToolBar::tr("Add torrent"));
        button->setToolTip(open->toolTip());
        button->setCursor(Qt::PointingHandCursor);
        button->setIconSize({14, 14});
        const auto paint = [button]
        {
            button->setIcon(lineIcon(QStringLiteral("<path d='M12 5v14M5 12h14'/>")
                , UIThemeManager::instance()->doseToken(QStringLiteral("onAccent"))));
        };
        paint();
        QObject::connect(UIThemeManager::instance(), &UIThemeManager::themeChanged, button, paint);
        QObject::connect(button, &QToolButton::clicked, open, &QAction::trigger);
        auto *action = new QWidgetAction(toolBar);
        action->setDefaultWidget(button);
        primaries().insert(toolBar, action);
        return action;
    }

    // Puts the toolbar back together from `original`, without hidden buttons.
    void rebuild(QToolBar *toolBar, const QList<QAction *> &original)
    {
        const QStringList hidden = hiddenButtons();
        for (QAction *a : original)
            toolBar->removeAction(a);
        if (QWidgetAction *primary = primaries().value(toolBar))
            toolBar->removeAction(primary);
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
        const bool dose = UIThemeManager::instance()->usesDoseTheme();
        for (QAction *a : keep)
        {
            if (dose && (a->objectName() == u"actionOpen"))
                toolBar->insertAction(before, primaryFor(toolBar, a));
            else
                toolBar->insertAction(before, a);
        }
    }
}

void DoseToolbar::install(QToolBar *toolBar, QMenu *contextMenu)
{
    if (!toolBar || !contextMenu)
        return;
    const QList<QAction *> original = toolBar->actions();

    toolBar->setIconSize({storedIconSize(), storedIconSize()});
    applySlim(toolBar, DoseToolbar::isSlim());

    contextMenu->addSeparator();
    QMenu *heights = contextMenu->addMenu(QToolBar::tr("Height"));
    auto *heightGroup = new QActionGroup(heights);
    for (const auto &[slim, name] : {std::pair {true, QToolBar::tr("Slim")}, std::pair {false, QToolBar::tr("Regular")}})
    {
        QAction *choice = heights->addAction(name);
        choice->setCheckable(true);
        choice->setChecked(slim == DoseToolbar::isSlim());
        heightGroup->addAction(choice);
        QObject::connect(choice, &QAction::triggered, toolBar, [toolBar, slim] { DoseToolbar::setSlim(toolBar, slim); });
    }
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
}

void DoseToolbar::polishFilter(QLineEdit *edit, QComboBox *columns)
{
    if (!edit || !columns || !UIThemeManager::instance()->usesDoseTheme())
        return;
    QWidget *window = edit->window();
    columns->hide();
    if (QWidget *row = columns->parentWidget())
    {
        if (auto *label = row->findChild<QLabel *>(QString(), Qt::FindDirectChildrenOnly))
            label->hide();
    }

    // The field: a rounded well holding a search button and the text box.
    auto *field = new QFrame;
    field->setObjectName(QStringLiteral("doseSearchField"));
    field->setFixedSize(440, 28); // the command list below matches this width
    auto *row = new QHBoxLayout(field);
    row->setContentsMargins(4, 0, 6, 0);
    row->setSpacing(2);
    auto *icon = new QToolButton;
    icon->setObjectName(QStringLiteral("doseSearchIcon"));
    icon->setIcon(UIThemeManager::instance()->getIcon(QStringLiteral("edit-find")));
    icon->setIconSize({16, 16});
    icon->setFixedSize(24, 22);
    icon->setCursor(Qt::PointingHandCursor);
    QObject::connect(UIThemeManager::instance(), &UIThemeManager::themeChanged, icon, [icon]
    {
        icon->setIcon(UIThemeManager::instance()->getIcon(QStringLiteral("edit-find")));
    });
    // The text box's own search icon is replaced by the button.
    const QList<QAction *> own = edit->actions();
    if (!own.isEmpty())
        edit->removeAction(own.first());
    edit->setMinimumWidth(0);
    edit->setMaximumWidth(QWIDGETSIZE_MAX);
    edit->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Fixed);
    edit->setFixedHeight(26);
    edit->setPlaceholderText(QToolBar::tr("Search torrents and commands  (Ctrl+K)"));
    row->addWidget(icon);
    row->addWidget(edit, 1);
    edit->show(); // moved from the toolbar: a reparented widget starts hidden

    auto *menu = new QMenu(field);
    const auto fill = [menu, columns]
    {
        menu->clear();
        QAction *heading = menu->addAction(QToolBar::tr("Filter by"));
        heading->setEnabled(false);
        menu->addSeparator();
        for (int i = 0; i < columns->count(); ++i)
        {
            QAction *choice = menu->addAction(columns->itemText(i));
            choice->setCheckable(true);
            choice->setChecked(i == columns->currentIndex());
            QObject::connect(choice, &QAction::triggered, columns, [columns, i] { columns->setCurrentIndex(i); });
        }
    };
    const auto describe = [icon, edit, columns]
    {
        icon->setToolTip(QToolBar::tr("Filter by: %1").arg(columns->currentText()));
        // One field for both: it filters the list and lists matching commands.
        edit->setPlaceholderText((columns->currentIndex() == 0) ? QToolBar::tr("Search torrents and commands  (Ctrl+K)")
            : QToolBar::tr("Search %1 and commands  (Ctrl+K)").arg(columns->currentText().toLower()));
    };
    describe();
    QObject::connect(columns, &QComboBox::currentIndexChanged, icon, describe);
    QObject::connect(icon, &QToolButton::clicked, field, [field, menu, fill]
    {
        fill();
        menu->popup(field->mapToGlobal(QPoint(0, field->height() + 4)));
    });

    DoseTitleBar::embedInTitle(qobject_cast<QMainWindow *>(window), field);
}

int DoseToolbar::iconSize()
{
    return storedIconSize();
}

bool DoseToolbar::isSlim()
{
    return SettingsStorage::instance()->loadValue<bool>(SLIM_KEY, true);
}

void DoseToolbar::setSlim(QToolBar *toolBar, const bool slim)
{
    SettingsStorage::instance()->storeValue(SLIM_KEY, slim);
    applySlim(toolBar, slim);
}

void DoseToolbar::setIconSize(QToolBar *toolBar, const int px)
{
    SettingsStorage::instance()->storeValue(ICON_SIZE_KEY, px);
    if (toolBar)
        toolBar->setIconSize({px, px});
}

void DoseToolbar::polishTabs(QTabWidget *tabs, QAction *rss, QAction *log)
{
    if (!tabs || !UIThemeManager::instance()->usesDoseTheme())
        return;
    tabs->setTabBarAutoHide(true);
    QTabBar *bar = tabs->tabBar();
    bar->setContextMenuPolicy(Qt::CustomContextMenu);
    QObject::connect(bar, &QWidget::customContextMenuRequested, bar, [bar, rss, log](const QPoint &pos)
    {
        QMenu menu {bar};
        QAction *heading = menu.addAction(QToolBar::tr("Show tabs"));
        heading->setEnabled(false);
        menu.addSeparator();
        for (QAction *a : {rss, log})
        {
            if (a && a->isVisible())
                menu.addAction(a);
        }
        menu.exec(bar->mapToGlobal(pos));
    });
}
