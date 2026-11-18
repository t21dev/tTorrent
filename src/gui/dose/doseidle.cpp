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

#include "doseidle.h"

#include <QAbstractItemModel>
#include <QEvent>
#include <QPointer>
#include <QTimer>
#include <QWidget>

namespace
{
    class Watcher final : public QObject
    {
    public:
        Watcher(QAbstractItemModel *model, QWidget *window)
            : QObject(window)
            , m_model {model}
            , m_window {window}
        {
            window->installEventFilter(this);
            update();
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if ((watched == m_window) && ((event->type() == QEvent::Show) || (event->type() == QEvent::Hide)
                || (event->type() == QEvent::WindowStateChange)))
            {
                // After the event, once visibility and state are final.
                QTimer::singleShot(0, this, [this] { update(); });
            }
            return QObject::eventFilter(watched, event);
        }

    private:
        void update()
        {
            if (!m_model)
                return;
            const bool paused = DoseIdle::windowHidden(m_window);
            m_model->setProperty("dosePaused", paused);
            if (!paused && m_model->property("dosePending").toBool())
            {
                m_model->setProperty("dosePending", false);
                if ((m_model->rowCount() > 0) && (m_model->columnCount() > 0))
                    emit m_model->dataChanged(m_model->index(0, 0), m_model->index(m_model->rowCount() - 1, m_model->columnCount() - 1));
            }
        }

        QPointer<QAbstractItemModel> m_model;
        QWidget *m_window = nullptr;
    };
}

bool DoseIdle::windowHidden(const QWidget *widget)
{
    const QWidget *window = widget ? widget->window() : nullptr;
    return window && (!window->isVisible() || window->isMinimized());
}

void DoseIdle::pauseWhileHidden(QAbstractItemModel *model, QWidget *view)
{
    if (!model || !view)
        return;
    // The view joins its main window after construction.
    QTimer::singleShot(0, view, [model, view] { new Watcher(model, view->window()); });
}
