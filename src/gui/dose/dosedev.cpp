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

#include "dosedev.h"

#include <functional>

#include <QAction>
#include <QApplication>
#include <QDialog>
#include <QDir>
#include <QLayout>
#include <QListWidget>
#include <QMainWindow>
#include <QMenu>
#include <QMenuBar>
#include <QPainter>
#include <QPixmap>
#include <QPushButton>
#include <QTabWidget>
#include <QTimer>
#include <QTreeView>

#include "base/global.h"
#include "dosethemeeditor.h"

namespace
{
    // A grab with a neutral grey behind translucent corners.
    void save(QWidget *widget, const QString &path)
    {
        widget->ensurePolished();
        if (!widget->isVisible())
            widget->resize(widget->sizeHint().expandedTo(widget->minimumSizeHint()));
        const QPixmap shot = widget->grab();
        QPixmap out {shot.size()};
        out.setDevicePixelRatio(shot.devicePixelRatio());
        out.fill(QColor(128, 128, 128));
        QPainter painter {&out};
        painter.drawPixmap(0, 0, shot);
        painter.end();
        out.save(path);
    }

    QList<QDialog *> openDialogs()
    {
        QList<QDialog *> list;
        for (QWidget *top : QApplication::topLevelWidgets())
        {
            if (auto *dialog = qobject_cast<QDialog *>(top); dialog && dialog->isVisible())
                list.append(dialog);
        }
        return list;
    }

    // Runs the steps one after another, `gap` ms apart.
    void chain(QObject *context, QList<std::function<void ()>> steps, const int gap)
    {
        if (steps.isEmpty())
            return;
        const auto step = steps.takeFirst();
        QTimer::singleShot(gap, context, [context, step, steps, gap]
        {
            step();
            chain(context, steps, gap);
        });
    }
}

void DoseDev::scheduleShots(QMainWindow *window)
{
    const QString dir = qEnvironmentVariable("TTORRENT_UISHOT_DIR");
    if (dir.isEmpty())
        return;
    QDir().mkpath(dir);

    const auto clickTab = [window](const QString &name)
    {
        for (QPushButton *tab : window->findChildren<QPushButton *>())
        {
            if (tab->property("doseTab").toBool() && tab->text().remove(u'&').contains(name, Qt::CaseInsensitive))
                tab->click();
        }
    };

    QList<std::function<void ()>> steps;
    // An Add torrent dialog opened from the command line: shoot it, then add.
    steps << [dir]
    {
        for (QDialog *dialog : openDialogs())
        {
            save(dialog, dir + u"/add-dialog.png");
            dialog->accept();
        }
    };
    steps << [window, dir]
    {
        // Select the first torrent so the details tabs have content.
        for (QTreeView *view : window->findChildren<QTreeView *>())
        {
            if (view->inherits("TransferListWidget") && view->model() && (view->model()->rowCount() > 0))
                view->setCurrentIndex(view->model()->index(0, 0));
        }
        save(window, dir + u"/main.png");
        for (QMenuBar *bar : window->findChildren<QMenuBar *>())
        {
            for (QAction *action : bar->actions())
            {
                if (QMenu *menu = action->menu(); menu && action->isVisible())
                    save(menu, dir + u"/menu-" + action->text().remove(u'&').toLower() + u".png");
            }
        }
    };
    for (const QString &tab : {u"General"_s, u"Trackers"_s, u"Peers"_s, u"HTTP"_s, u"Content"_s, u"Speed"_s})
    {
        steps << [window, dir, clickTab, tab]
        {
            clickTab(tab);
            QApplication::processEvents();
            save(window, dir + u"/tab-" + tab.toLower() + u".png");
        };
    }
    // Keyboard shortcuts sheet and the command palette.
    steps << [window]
    {
        if (auto *sheet = window->findChild<QAction *>(u"doseActionShortcuts"_s))
            sheet->trigger();
    };
    steps << [window, dir]
    {
        for (QDialog *dialog : openDialogs())
        {
            save(dialog, dir + u"/shortcuts.png");
            dialog->close();
        }
        if (auto *palette = window->findChild<QAction *>(u"doseActionPalette"_s))
            palette->trigger();
    };
    steps << [dir]
    {
        for (QWidget *top : QApplication::topLevelWidgets())
        {
            if (top->isVisible() && (top->objectName() == u"dosePalette"))
            {
                save(top, dir + u"/palette.png");
                top->close();
            }
        }
    };
    steps << [window]
    {
        if (auto *options = window->findChild<QAction *>(u"actionOptions"_s))
            options->trigger();
    };
    steps << [dir]
    {
        for (QDialog *dialog : openDialogs())
        {
            save(dialog, dir + u"/options.png");
            // The last section (Advanced) too.
            if (auto *sections = dialog->findChild<QListWidget *>(u"tabSelection"_s))
            {
                sections->setCurrentRow(sections->count() - 1);
                QApplication::processEvents();
                save(dialog, dir + u"/options-advanced.png");
                sections->setCurrentRow(1);
                QApplication::processEvents();
                save(dialog, dir + u"/options-downloads.png");
            }
            dialog->close();
        }
    };
    steps << [window]
    {
        if (auto *about = window->findChild<QAction *>(u"actionAbout"_s))
            about->trigger();
    };
    steps << [dir]
    {
        for (QDialog *dialog : openDialogs())
        {
            save(dialog, dir + u"/about.png");
            dialog->close();
        }
        DoseThemeEditor editor {u"dark"_s};
        editor.resize(900, 620);
        editor.ensurePolished();
        editor.layout()->activate();
        editor.grab().save(dir + u"/theme-editor.png");
    };
    // Every other dialog the menus open, one at a time.
    for (const QString &name : {u"actionStatistics"_s, u"actionDownloadFromURL"_s, u"actionCreateTorrent"_s
            , u"actionSetGlobalSpeedLimits"_s, u"actionManageCookies"_s})
    {
        steps << [window, name]
        {
            if (auto *action = window->findChild<QAction *>(name))
                action->trigger();
        };
        steps << [dir, name]
        {
            for (QDialog *dialog : openDialogs())
            {
                save(dialog, dir + u"/dialog-" + name.mid(6).toLower() + u".png");
                dialog->close();
            }
        };
    }
    // RSS and the log as tabs of the main window.
    for (const QString &name : {u"actionRSSReader"_s, u"actionExecutionLogs"_s})
    {
        steps << [window, dir, name]
        {
            if (auto *action = window->findChild<QAction *>(name); action && !action->isChecked())
                action->trigger();
            for (QTabWidget *tabs : window->findChildren<QTabWidget *>())
            {
                if (tabs->count() > 1)
                    tabs->setCurrentIndex(tabs->count() - 1);
            }
            QApplication::processEvents();
            save(window, dir + u"/main-" + name.mid(6).toLower() + u".png");
        };
    }
    steps << [] { QApplication::quit(); };
    chain(window, steps, 1500);
}
