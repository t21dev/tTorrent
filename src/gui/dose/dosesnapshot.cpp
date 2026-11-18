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

#include "dosesnapshot.h"

#include <QAbstractItemView>
#include <QApplication>
#include <QDir>
#include <QEvent>
#include <QFile>
#include <QGraphicsOpacityEffect>
#include <QHeaderView>
#include <QLabel>
#include <QPixmap>
#include <QPointer>
#include <QPropertyAnimation>
#include <QTimer>

#include "base/bittorrent/session.h"
#include "base/global.h"
#include "base/profile.h"
#include "dosepaint.h"

namespace
{
    QString snapshotPath()
    {
        return Profile::instance()->location(SpecialFolder::Cache).data() + u"/dose-list.png"_s;
    }

    // The picture over the empty list; any click or key on the list removes it.
    class Cover final : public QLabel
    {
    public:
        explicit Cover(QWidget *viewport)
            : QLabel(viewport)
        {
            setObjectName(u"doseSnapshotCover"_s);
            setAttribute(Qt::WA_TransparentForMouseEvents);
            setAlignment(Qt::AlignLeft | Qt::AlignTop);
            setAutoFillBackground(true);
            setBackgroundRole(QPalette::Base);
        }

        void leave()
        {
            if (m_leaving)
                return;
            m_leaving = true;
            if (!DosePaint::animationsEnabled() || !isVisible())
            {
                deleteLater();
                return;
            }
            auto *effect = new QGraphicsOpacityEffect(this);
            setGraphicsEffect(effect);
            auto *fade = new QPropertyAnimation(effect, "opacity", this);
            fade->setDuration(180);
            fade->setStartValue(1.0);
            fade->setEndValue(0.0);
            fade->setEasingCurve(QEasingCurve::OutCubic);
            connect(fade, &QAbstractAnimation::finished, this, &QObject::deleteLater);
            fade->start();
        }

    private:
        bool m_leaving = false;
    };

    class Watcher final : public QObject
    {
    public:
        Watcher(QAbstractItemView *view, Cover *cover)
            : QObject(view)
            , m_view {view}
            , m_cover {cover}
        {
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            switch (event->type())
            {
            case QEvent::Resize:
                if (m_cover)
                    m_cover->resize(m_view->viewport()->size());
                break;
            case QEvent::MouseButtonPress:
            case QEvent::KeyPress:
            case QEvent::Wheel:
                if (m_cover)
                    m_cover->leave();
                break;
            default:
                break;
            }
            return QObject::eventFilter(watched, event);
        }

    private:
        QAbstractItemView *m_view;
        QPointer<Cover> m_cover;
    };

    // Saves the picture whenever the list hides: when tTorrent quits (the
    // main window hides before anything is deleted) or goes to the tray.
    class Saver final : public QObject
    {
    public:
        using QObject::QObject;

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if (event->type() == QEvent::Hide)
            {
                auto *view = static_cast<QAbstractItemView *>(watched);
                const QString path = snapshotPath();
                // Only a loaded, non-empty list is worth showing next time.
                if (!BitTorrent::Session::instance()->isRestored() || !view->model() || (view->model()->rowCount() == 0)
                    || (view->viewport()->findChild<QLabel *>(u"doseSnapshotCover"_s) != nullptr))
                {
                    if (BitTorrent::Session::instance()->isRestored())
                        QFile::remove(path);
                }
                else
                {
                    QDir().mkpath(QFileInfo(path).absolutePath());
                    view->viewport()->grab().save(path, "PNG");
                }
            }
            return QObject::eventFilter(watched, event);
        }
    };

}

void DoseSnapshot::attach(QAbstractItemView *view)
{
    if (!view || !DosePaint::active())
        return;
    view->installEventFilter(new Saver(view));
    if (BitTorrent::Session::instance()->isRestored())
        return;

    QPixmap picture;
    if (!picture.load(snapshotPath()))
        return;
    // The picture was taken at the screen's scale; show it at the same size.
    picture.setDevicePixelRatio(view->devicePixelRatioF());
    auto *cover = new Cover(view->viewport());
    cover->setPixmap(picture);
    cover->resize(view->viewport()->size());
    cover->show();
    auto *watcher = new Watcher(view, cover);
    view->viewport()->installEventFilter(watcher);
    view->installEventFilter(watcher);

    const QPointer<Cover> guard {cover};
    const auto leave = [guard]
    {
        // One turn for the model's rows to be laid out and painted.
        QTimer::singleShot(80, guard, [guard] { if (guard) guard->leave(); });
    };
    QObject::connect(BitTorrent::Session::instance(), &BitTorrent::Session::restored, cover, leave);
    // Never longer than this, whatever happens.
    QTimer::singleShot(15000, cover, [guard] { if (guard) guard->leave(); });
}
