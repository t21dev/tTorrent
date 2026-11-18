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
    for (const QString &tab : {u"General"_s, u"Trackers"_s, u"Peers"_s, u"Content"_s, u"Speed"_s})
    {
        steps << [window, dir, clickTab, tab]
        {
            clickTab(tab);
            QApplication::processEvents();
            save(window, dir + u"/tab-" + tab.toLower() + u".png");
        };
    }
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
    steps << [] { QApplication::quit(); };
    chain(window, steps, 1500);
}
