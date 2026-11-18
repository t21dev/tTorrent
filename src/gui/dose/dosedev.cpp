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

#include <QAction>
#include <QApplication>
#include <QDialog>
#include <QDir>
#include <QLayout>
#include <QMainWindow>
#include <QMenu>
#include <QMenuBar>
#include <QPainter>
#include <QPixmap>
#include <QPushButton>
#include <QTimer>

#include "base/global.h"
#include "dosethemeeditor.h"

namespace
{
    // A grab with the window's own background behind translucent corners.
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
}

void DoseDev::scheduleShots(QMainWindow *window)
{
    const QString dir = qEnvironmentVariable("TTORRENT_UISHOT_DIR");
    if (dir.isEmpty())
        return;
    QDir().mkpath(dir);

    QTimer::singleShot(4000, window, [window, dir]
    {
        save(window, dir + u"/main.png");
        for (QMenuBar *bar : window->findChildren<QMenuBar *>())
        {
            for (QAction *action : bar->actions())
            {
                if (QMenu *menu = action->menu())
                {
                    QString name = action->text();
                    name.remove(u'&');
                    save(menu, dir + u"/menu-" + name.toLower() + u".png");
                }
            }
        }
        // The Speed tab of the details pane.
        for (QPushButton *tab : window->findChildren<QPushButton *>())
        {
            if (tab->property("doseTab").toBool() && tab->text().contains(u"Speed"))
                tab->click();
        }
        QApplication::processEvents();
        save(window, dir + u"/main-speed.png");
        if (auto *options = window->findChild<QAction *>(u"actionOptions"_s))
            options->trigger();
        const auto shootDialogs = [](const QString &path)
        {
            for (QWidget *top : QApplication::topLevelWidgets())
            {
                if (auto *dialog = qobject_cast<QDialog *>(top); dialog && dialog->isVisible())
                {
                    save(dialog, path);
                    dialog->close();
                }
            }
        };
        QTimer::singleShot(1500, window, [window, dir, shootDialogs]
        {
            shootDialogs(dir + u"/options.png");
            if (auto *about = window->findChild<QAction *>(u"actionAbout"_s))
                about->trigger();
            QTimer::singleShot(1200, window, [dir, shootDialogs]
            {
                shootDialogs(dir + u"/about.png");
                // The theme editor, rendered without showing it.
                DoseThemeEditor editor {u"dark"_s};
                editor.resize(900, 620);
                editor.ensurePolished();
                editor.layout()->activate();
                editor.grab().save(dir + u"/theme-editor.png");
                QTimer::singleShot(300, qApp, &QApplication::quit);
            });
        });
    });
}
