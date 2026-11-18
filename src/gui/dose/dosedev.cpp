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
#include <QFile>
#include <QLayout>
#include <QLabel>
#include <QListWidget>
#include <QMouseEvent>
#include <QMainWindow>
#include <QMenu>
#include <QMenuBar>
#include <QPainter>
#include <QPixmap>
#include <QPushButton>
#include <QScrollArea>
#include <QTabWidget>
#include <QTimer>
#include <QTreeView>

#include "base/bittorrent/session.h"
#include "base/bittorrent/torrent.h"
#include "base/global.h"
#include "base/path.h"
#include "base/tag.h"
#include "dosebackup.h"
#include "doserelocate.h"
#include "doserename.h"
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

namespace
{
    // QApplication::quit() alone doesn't shut tTorrent down; File › Exit does.
    void quitApp()
    {
        for (QWidget *top : QApplication::topLevelWidgets())
        {
            if (auto *main = qobject_cast<QMainWindow *>(top))
            {
                if (auto *exit = main->findChild<QAction *>(u"actionExit"_s))
                {
                    exit->trigger();
                    return;
                }
            }
        }
        QApplication::quit();
    }

    // Self-test of features whose menus need dialogs (ttorrent/VERIFY.md):
    // rename category and tag, back up, relocate. Runs on the first torrent of
    // a scratch profile once the session is ready, writes PASS/FAIL lines to
    // <TTORRENT_SELFTEST_DIR>/selftest.txt and quits. TTORRENT_SELFTEST_MOVED
    // is a folder holding a copy of that torrent's files under a renamed top folder.
    void runSelfTest(QMainWindow *window, const QString &dir)
    {
        QDir().mkpath(dir);
        auto *lines = new QStringList;
        const auto check = [lines](const QString &name, const bool ok, const QString &detail = {})
        {
            lines->append((ok ? u"PASS "_s : u"FAIL "_s) + name + (detail.isEmpty() ? QString() : (u" - "_s + detail)));
        };
        const auto finish = [lines, dir]
        {
            QFile out {dir + u"/selftest.txt"_s};
            if (out.open(QIODevice::WriteOnly | QIODevice::Text))
                out.write(lines->join(QChar(u'\n')).toUtf8() + QByteArray(1, '\n'));
            delete lines;
            quitApp();
        };
        const auto start = [window, check, finish]
        {
            BitTorrent::Session *session = BitTorrent::Session::instance();
            if (session->torrents().isEmpty())
            {
                check(u"setup"_s, false, u"no torrent in the profile"_s);
                return finish();
            }
            BitTorrent::Torrent *torrent = session->torrents().first();

            // After a restore: report what the torrent looks like now.
            if (qEnvironmentVariableIsSet("TTORRENT_SELFTEST_REPORT"))
            {
                QStringList tags;
                for (const Tag &tag : torrent->tags())
                    tags.append(tag.toString());
                check(u"state"_s, true, u"save path "_s + torrent->savePath().toString() + u"; category "_s + torrent->category()
                    + u"; tags "_s + tags.join(u", "_s) + u"; categories "_s + session->categories().join(u", "_s));
                return finish();
            }

            // Second run, after the files were moved: relocate only.
            const QString moved = qEnvironmentVariable("TTORRENT_SELFTEST_MOVED");
            if (!moved.isEmpty())
            {
                const int found = DoseRelocate::relocateTo(torrent, moved);
                check(u"relocate finds files"_s, found == torrent->filesCount(), QString::number(found) + u" of "_s + QString::number(torrent->filesCount()));
                // The check runs in the background; look again once it's done.
                QTimer::singleShot(12000, window, [torrent, moved, check, finish]
                {
                    check(u"relocate: save path"_s, torrent->savePath() == Path(moved), torrent->savePath().toString());
                    check(u"relocate: complete after check"_s, torrent->progress() >= 0.999, QString::number(torrent->progress() * 100, 'f', 1) + u" %"_s);
                    finish();
                });
                return;
            }

            session->addCategory(u"Films"_s);
            session->addCategory(u"Films/4K"_s);
            torrent->setCategory(u"Films/4K"_s);
            const QString catError = DoseRename::renameCategory(u"Films"_s, u"Movies"_s);
            const QStringList cats = session->categories();
            check(u"rename category"_s, catError.isEmpty() && cats.contains(u"Movies"_s) && cats.contains(u"Movies/4K"_s) && !cats.contains(u"Films"_s)
                && !cats.contains(u"Films/4K"_s) && (torrent->category() == u"Movies/4K"), u"categories: "_s + cats.join(u", "_s) + u"; torrent: "_s + torrent->category() + u" "_s + catError);

            const Tag old {u"old"_s};
            session->addTag(old);
            torrent->addTag(old);
            const QString tagError = DoseRename::renameTag(old, u"new"_s);
            check(u"rename tag"_s, tagError.isEmpty() && torrent->hasTag(Tag(u"new"_s)) && !torrent->hasTag(old) && !session->hasTag(old), tagError);
            check(u"rename to an existing tag is refused"_s, !DoseRename::renameTag(Tag(u"new"_s), u"new"_s).isEmpty());

            DoseBackup::backupSoon(window, u"selftest"_s, [check, finish](const QString &backup)
            {
                const int saved = backup.isEmpty() ? 0 : int(QDir(backup + u"/data/BT_backup"_s).entryList({u"*.fastresume"_s}, QDir::Files).size());
                check(u"backup"_s, QFile::exists(backup + u"/config/tTorrent.ini"_s) && (saved >= 1), backup + u" ("_s + QString::number(saved) + u" torrents)"_s);
                finish();
            });
        };
        // Wait up to 30 s for a torrent added on the command line, and for its first check.
        auto *poll = new QTimer(window);
        auto *tries = new int(0);
        QObject::connect(poll, &QTimer::timeout, window, [poll, tries, start]
        {
            const QList<BitTorrent::Torrent *> torrents = BitTorrent::Session::instance()->torrents();
            const bool ready = BitTorrent::Session::instance()->isRestored() && !torrents.isEmpty() && torrents.first()->hasMetadata()
                && !torrents.first()->isChecking();
            if (!ready && (++*tries < 30))
                return;
            poll->stop();
            delete tries;
            start();
        });
        poll->start(1000);
    }
}

void DoseDev::scheduleShots(QMainWindow *window)
{
    if (const QString test = qEnvironmentVariable("TTORRENT_SELFTEST_DIR"); !test.isEmpty())
        return runSelfTest(window, test);
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
    // Give the Add dialog's health check a few seconds to hear from trackers.
    steps << [] {} << [] {} << [] {};
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
            // The bottom of the first page: Updates and Backup.
            if (auto *sections = dialog->findChild<QListWidget *>(u"tabSelection"_s))
            {
                sections->setCurrentRow(0);
                QApplication::processEvents();
            }
            if (auto *group = dialog->findChild<QWidget *>(u"doseBackup"_s))
            {
                for (QWidget *p = group->parentWidget(); p; p = p->parentWidget())
                {
                    if (auto *area = qobject_cast<QScrollArea *>(p))
                    {
                        area->ensureWidgetVisible(group, 0, 0);
                        break;
                    }
                }
                QApplication::processEvents();
                save(dialog, dir + u"/options-backup.png");
            }
            // The last section (Advanced) too.
            if (auto *sections = dialog->findChild<QListWidget *>(u"tabSelection"_s))
            {
                sections->setCurrentRow(sections->count() - 1);
                QApplication::processEvents();
                save(dialog, dir + u"/options-advanced.png");
                sections->setCurrentRow(1);
                QApplication::processEvents();
                save(dialog, dir + u"/options-downloads.png");
                if (auto *group = dialog->findChild<QWidget *>(u"doseAutomation"_s))
                {
                    for (QWidget *p = group->parentWidget(); p; p = p->parentWidget())
                    {
                        if (auto *area = qobject_cast<QScrollArea *>(p))
                        {
                            area->ensureWidgetVisible(group, 0, 0);
                            break;
                        }
                    }
                    QApplication::processEvents();
                    save(dialog, dir + u"/options-automation.png");
                }
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
    // The memory card, opened as a click on the status bar's Memory label would.
    steps << [window]
    {
        for (QLabel *label : window->findChildren<QLabel *>())
        {
            if (label->toolTip().contains(u"graph since start"))
            {
                QMouseEvent release {QEvent::MouseButtonRelease, QPointF(4, 4), label->mapToGlobal(QPointF(4, 4)), Qt::LeftButton, Qt::NoButton, Qt::NoModifier};
                QApplication::sendEvent(label, &release);
            }
        }
    };
    steps << [dir]
    {
        for (QWidget *top : QApplication::topLevelWidgets())
        {
            if (top->isVisible() && (top->objectName() == u"doseMemoryPopup"))
            {
                save(top, dir + u"/memory.png");
                top->close();
            }
        }
    };
    steps << [window]
    {
        if (auto *pair = window->findChild<QAction *>(u"doseActionPair"_s))
            pair->trigger();
    };
    steps << [dir]
    {
        for (QDialog *dialog : openDialogs())
        {
            save(dialog, dir + u"/pair.png");
            dialog->close();
        }
    };
    // Help › Check for updates: the card or the message it ends with.
    steps << [window]
    {
        if (auto *check = window->findChild<QAction *>(u"doseActionUpdates"_s))
            check->trigger();
    };
    steps << [] {} << [] {};
    steps << [window, dir]
    {
        save(window, dir + u"/update-window.png");
        for (QDialog *dialog : openDialogs())
        {
            save(dialog, dir + u"/update-message.png");
            dialog->close();
        }
    };
    steps << [] { quitApp(); };
    chain(window, steps, 1500);
}
