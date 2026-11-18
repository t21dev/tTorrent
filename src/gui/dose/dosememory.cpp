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

#include "dosememory.h"

#include <QDateTime>
#include <QElapsedTimer>
#include <QEvent>
#include <QFrame>
#include <QGridLayout>
#include <QLabel>
#include <QMainWindow>
#include <QMouseEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPixmapCache>
#include <QPointer>
#include <QPushButton>
#include <QTimer>
#include <QTimeZone>
#include <QVBoxLayout>

#ifdef Q_OS_WIN
#include <windows.h>
#include <malloc.h>
#include <psapi.h>
#endif

#include "base/bittorrent/session.h"
#include "base/global.h"
#include "base/logger.h"
#include "base/utils/misc.h"
#include "dosepaint.h"

namespace
{
    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseMemory", text);
    }

    struct Sample
    {
        qint64 msecs;      // since the process started
        qint64 workingSet; // private working set
        qint64 reserved;   // private bytes
    };

    const int SAMPLE_SECONDS = 30;
    const int MAX_SAMPLES = (24 * 60 * 60) / SAMPLE_SECONDS;

    QList<Sample> &samples()
    {
        static QList<Sample> list;
        return list;
    }

    qint64 &peak()
    {
        static qint64 value = 0;
        return value;
    }

    // When Windows started this process, so start-up times include loading the exe and Qt.
    QDateTime processStart()
    {
#ifdef Q_OS_WIN
        FILETIME created {}, exited {}, kernel {}, user {};
        if (::GetProcessTimes(::GetCurrentProcess(), &created, &exited, &kernel, &user))
        {
            const qint64 ticks = (qint64(created.dwHighDateTime) << 32) | created.dwLowDateTime;
            // FILETIME counts 100 ns since 1601-01-01.
            return QDateTime::fromMSecsSinceEpoch((ticks / 10000) - 11644473600000LL, QTimeZone::UTC);
        }
#endif
        return QDateTime::currentDateTimeUtc();
    }

    qint64 sinceStart()
    {
        static const QDateTime start = processStart();
        return start.msecsTo(QDateTime::currentDateTimeUtc());
    }

    void sample()
    {
        const Sample s {sinceStart(), DoseMemory::privateWorkingSet(), DoseMemory::privateBytes()};
        if (s.workingSet < 0)
            return;
        peak() = std::max(peak(), s.workingSet);
        samples().append(s);
        if (samples().size() > MAX_SAMPLES)
            samples().removeFirst();
    }

    QString duration(const qint64 msecs)
    {
        const qint64 minutes = msecs / 60000;
        if (minutes < 1)
            return tr("%1 s").arg(msecs / 1000);
        if (minutes < 60)
            return tr("%1 min").arg(minutes);
        return tr("%1 h %2 min").arg(QString::number(minutes / 60), QString::number(minutes % 60));
    }

    // The graph: private working set over time, the peak marked.
    class Graph final : public QWidget
    {
    public:
        Graph()
        {
            setMinimumSize(320, 110);
        }

    protected:
        void paintEvent(QPaintEvent *) override
        {
            QPainter p {this};
            p.setRenderHint(QPainter::Antialiasing);
            const QRectF area = QRectF(rect()).adjusted(1, 8, -1, -16);
            const QList<Sample> &list = samples();
            const QColor line = DosePaint::token(u"line"_s);
            const QColor faint = DosePaint::token(u"faint"_s);
            const QColor accent = DosePaint::token(u"accent"_s);
            QFont small = font();
            small.setPixelSize(std::max(9, font().pixelSize() - 2));
            p.setFont(small);
            p.setPen(line);
            p.drawLine(area.bottomLeft(), area.bottomRight());
            if (list.size() < 2)
            {
                p.setPen(faint);
                p.drawText(area, Qt::AlignCenter, tr("The graph fills in as tTorrent runs."));
                return;
            }
            qint64 top = 0;
            for (const Sample &s : list)
                top = std::max(top, s.workingSet);
            top = qint64(top * 1.15);
            const qint64 first = list.first().msecs;
            const qint64 span = std::max<qint64>(1, list.last().msecs - first);
            const auto point = [&](const Sample &s)
            {
                return QPointF(area.left() + (area.width() * (s.msecs - first) / span)
                    , area.bottom() - (area.height() * s.workingSet / qreal(top)));
            };
            QPainterPath path {point(list.first())};
            for (const Sample &s : list)
                path.lineTo(point(s));
            QPainterPath fill = path;
            fill.lineTo(area.right(), area.bottom());
            fill.lineTo(area.left(), area.bottom());
            QColor wash = accent;
            wash.setAlphaF(0.14f);
            p.fillPath(fill, wash);
            p.setPen(QPen(accent, 1.6));
            p.drawPath(path);
            p.setPen(faint);
            p.drawText(QRectF(area.left(), area.bottom() + 2, area.width(), 14), Qt::AlignLeft | Qt::AlignTop
                , tr("%1 ago").arg(duration(list.last().msecs - first)));
            p.drawText(QRectF(area.left(), area.bottom() + 2, area.width(), 14), Qt::AlignRight | Qt::AlignTop, tr("now"));
            p.drawText(QRectF(area.left(), 0, area.width(), 12), Qt::AlignRight | Qt::AlignTop
                , Utils::Misc::friendlyUnit(qint64(top / 1.15)));
        }
    };

    void showCard(QLabel *anchor)
    {
        sample();
        auto *card = new QFrame(anchor->window(), Qt::Popup | Qt::FramelessWindowHint | Qt::NoDropShadowWindowHint);
        card->setObjectName(u"doseMemoryPopup"_s);
        card->setAttribute(Qt::WA_DeleteOnClose);
        card->setAttribute(Qt::WA_TranslucentBackground);
        auto *outer = new QVBoxLayout(card);
        outer->setContentsMargins(0, 0, 0, 0);
        auto *body = new QFrame;
        body->setObjectName(u"doseMemoryCard"_s);
        outer->addWidget(body);
        auto *column = new QVBoxLayout(body);
        column->setContentsMargins(14, 12, 14, 12);
        column->setSpacing(8);

        auto *heading = new QLabel(tr("Memory since start"));
        heading->setObjectName(u"doseCardTitle"_s);
        column->addWidget(heading);
        auto *facts = new QGridLayout;
        facts->setHorizontalSpacing(18);
        facts->setVerticalSpacing(2);
        const auto fact = [facts](const int col, const QString &key, QLabel *value)
        {
            auto *k = new QLabel(key);
            k->setObjectName(u"doseMuted"_s);
            value->setObjectName(u"doseMemoryValue"_s);
            facts->addWidget(k, 0, col);
            facts->addWidget(value, 1, col);
        };
        auto *now = new QLabel;
        auto *top = new QLabel;
        auto *reserved = new QLabel;
        auto *uptime = new QLabel;
        fact(0, tr("Now"), now);
        fact(1, tr("Peak"), top);
        fact(2, tr("Reserved"), reserved);
        fact(3, tr("Running for"), uptime);
        column->addLayout(facts);
        auto *graph = new Graph;
        column->addWidget(graph);

        auto *row = new QHBoxLayout;
        auto *note = new QLabel(tr("Now is Task Manager's Memory column. Reserved includes memory not in use right now."));
        note->setObjectName(u"doseMuted"_s);
        note->setWordWrap(true);
        row->addWidget(note, 1);
        auto *trim = new QPushButton(tr("Return free memory"));
        trim->setToolTip(tr("Gives memory tTorrent has freed back to Windows. tTorrent also does this after a minute in the background."));
        row->addWidget(trim);
        column->addLayout(row);

        const auto refresh = [now, top, reserved, uptime, graph]
        {
            const Sample &s = samples().last();
            now->setText(Utils::Misc::friendlyUnit(s.workingSet));
            top->setText(Utils::Misc::friendlyUnit(peak()));
            reserved->setText(Utils::Misc::friendlyUnit(s.reserved));
            uptime->setText(duration(sinceStart()));
            graph->update();
        };
        refresh();
        QObject::connect(trim, &QPushButton::clicked, card, [trim, refresh]
        {
            const qint64 freed = DoseMemory::returnFreeMemory();
            sample();
            refresh();
            trim->setText((freed > 0) ? tr("Returned %1").arg(Utils::Misc::friendlyUnit(freed)) : tr("Nothing to return"));
        });
        auto *timer = new QTimer(card);
        QObject::connect(timer, &QTimer::timeout, card, [refresh] { sample(); refresh(); });
        timer->start(5000);

        card->adjustSize();
        const QPoint corner = anchor->mapToGlobal(QPoint(0, 0));
        card->move(corner.x(), corner.y() - card->height() - 6);
        card->show();
    }

    // After a minute hidden or minimized, give freed memory back once.
    class IdleReturn final : public QObject
    {
    public:
        explicit IdleReturn(QMainWindow *window)
            : QObject(window)
            , m_window {window}
        {
            m_timer.setSingleShot(true);
            m_timer.setInterval(60 * 1000);
            connect(&m_timer, &QTimer::timeout, this, [this]
            {
                if (m_window->isVisible() && !m_window->isMinimized())
                    return;
                const qint64 freed = DoseMemory::returnFreeMemory();
                if (freed > (1024 * 1024))
                    LogMsg(tr("Returned %1 of free memory to Windows while in the background.").arg(Utils::Misc::friendlyUnit(freed)));
            });
            window->installEventFilter(this);
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if ((watched == m_window) && ((event->type() == QEvent::Hide) || (event->type() == QEvent::WindowStateChange)
                || (event->type() == QEvent::Show)))
            {
                QTimer::singleShot(0, this, [this]
                {
                    if (!m_window->isVisible() || m_window->isMinimized())
                        m_timer.start();
                    else
                        m_timer.stop();
                });
            }
            return QObject::eventFilter(watched, event);
        }

    private:
        QMainWindow *m_window = nullptr;
        QTimer m_timer;
    };

    // Start-up timing for the log: to the window, and until every torrent is ready.
    class StartupClock final : public QObject
    {
    public:
        explicit StartupClock(QMainWindow *window)
            : QObject(window)
            , m_window {window}
        {
            window->installEventFilter(this);
            BitTorrent::Session *session = BitTorrent::Session::instance();
            const auto report = [session]
            {
                LogMsg(tr("Start-up: %1 torrents ready %2 s after launch (memory %3).")
                    .arg(QString::number(session->torrents().size()), QString::number(sinceStart() / 1000.0, 'f', 2)
                        , Utils::Misc::friendlyUnit(DoseMemory::privateWorkingSet())));
            };
            if (session->isRestored())
                report();
            else
                connect(session, &BitTorrent::Session::restored, this, report);
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if ((watched == m_window) && (event->type() == QEvent::Show) && (m_windowMsecs < 0))
            {
                m_windowMsecs = sinceStart();
                LogMsg(tr("Start-up: window shown %1 s after launch.").arg(QString::number(m_windowMsecs / 1000.0, 'f', 2)));
            }
            return QObject::eventFilter(watched, event);
        }

    private:
        QMainWindow *m_window = nullptr;
        qint64 m_windowMsecs = -1;
    };

    class LabelClick final : public QObject
    {
    public:
        using QObject::QObject;

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if (event->type() == QEvent::MouseButtonRelease)
            {
                if (auto *label = qobject_cast<QLabel *>(watched); label && (static_cast<QMouseEvent *>(event)->button() == Qt::LeftButton))
                {
                    showCard(label);
                    return true;
                }
            }
            return QObject::eventFilter(watched, event);
        }
    };
}

qint64 DoseMemory::privateWorkingSet()
{
#ifdef Q_OS_WIN
    // PROCESS_MEMORY_COUNTERS_EX2's layout (Windows 10 1809+).
    struct
    {
        PROCESS_MEMORY_COUNTERS_EX ex;
        SIZE_T privateWorkingSetSize;
        SIZE_T sharedCommitUsage;
    } counters {};
    counters.ex.cb = sizeof(counters);
    if (::GetProcessMemoryInfo(::GetCurrentProcess(), reinterpret_cast<PROCESS_MEMORY_COUNTERS *>(&counters), counters.ex.cb))
        return static_cast<qint64>(counters.privateWorkingSetSize);
#endif
    return -1;
}

qint64 DoseMemory::privateBytes()
{
#ifdef Q_OS_WIN
    PROCESS_MEMORY_COUNTERS_EX counters {};
    counters.cb = sizeof(counters);
    if (::GetProcessMemoryInfo(::GetCurrentProcess(), reinterpret_cast<PROCESS_MEMORY_COUNTERS *>(&counters), counters.cb))
        return static_cast<qint64>(counters.PrivateUsage);
#endif
    return -1;
}

qint64 DoseMemory::returnFreeMemory()
{
    const qint64 before = privateBytes();
    QPixmapCache::clear();
#ifdef Q_OS_WIN
    // The CRT's free blocks, then every heap's free pages, back to Windows.
    _heapmin();
    HANDLE heaps[64];
    const DWORD count = ::GetProcessHeaps(64, heaps);
    for (DWORD i = 0; (i < count) && (i < 64); ++i)
        ::HeapCompact(heaps[i], 0);
#endif
    const qint64 after = privateBytes();
    return ((before >= 0) && (after >= 0)) ? std::max<qint64>(0, before - after) : 0;
}

void DoseMemory::install(QMainWindow *window)
{
    if (!window)
        return;
    // Qt keeps up to 10 MB of drawn pixmaps by default; icons and the
    // speed graph redraw cheaply, so 4 MB is plenty.
    QPixmapCache::setCacheLimit(4 * 1024);
    sample();
    auto *timer = new QTimer(window);
    QObject::connect(timer, &QTimer::timeout, window, [] { sample(); });
    timer->start(SAMPLE_SECONDS * 1000);
    new IdleReturn(window);
    new StartupClock(window);
}

void DoseMemory::attachToLabel(QLabel *label)
{
    if (!label)
        return;
    label->setCursor(Qt::PointingHandCursor);
    label->setToolTip(tr("Memory tTorrent uses (Task Manager's Memory column). Click for the graph since start."));
    label->installEventFilter(new LabelClick(label));
}
