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

// tTorrent's look for the Speed tab (DESIGN.md › Speed graph), as a member of
// qBittorrent's SpeedPlotView so it reads the same data, graphs and period:
// smooth lines with a fading fill for download (accent) and upload (`ok`),
// a dashed hairline grid, axis labels in DM Mono, a dot on the latest value,
// and a legend with the live values. Other graphs (payload, overhead, DHT,
// tracker) keep their own colours as thin lines.

#include "gui/properties/speedplotview.h"

#include <cmath>

#include <QLinearGradient>
#include <QPainter>
#include <QPainterPath>

#include "base/global.h"
#include "base/utils/misc.h"
#include "dosepaint.h"

namespace
{
    // A round top for the axis: 1, 2, 2.5 or 5 times a power of ten.
    double niceMax(const double value)
    {
        if (value <= 0)
            return 1024;
        const double magnitude = std::pow(10.0, std::floor(std::log10(value)));
        for (const double step : {1.0, 2.0, 2.5, 5.0, 10.0})
        {
            if ((step * magnitude) >= value)
                return step * magnitude;
        }
        return 10 * magnitude;
    }

    QString durationLabel(const std::chrono::milliseconds duration)
    {
        const qint64 minutes = std::chrono::duration_cast<std::chrono::minutes>(duration).count();
        if (minutes >= 60)
            return QString::number(minutes / 60) + u'h';
        return QString::number(std::max<qint64>(1, minutes)) + u'm';
    }

    QPainterPath smooth(const QList<QPointF> &points)
    {
        QPainterPath path;
        if (points.isEmpty())
            return path;
        path.moveTo(points.first());
        for (int i = 1; i < points.size(); ++i)
        {
            const QPointF &a = points[i - 1];
            const QPointF &b = points[i];
            const qreal mid = (a.x() + b.x()) / 2;
            path.cubicTo(QPointF(mid, a.y()), QPointF(mid, b.y()), b);
        }
        return path;
    }
}

void SpeedPlotView::paintDose()
{
    QPainter painter {viewport()};
    painter.setRenderHint(QPainter::Antialiasing);
    const QRect full = viewport()->rect();
    painter.fillRect(full, DosePaint::token(u"bg"_s));

    const QFont numbers = DosePaint::numberFont(font());
    const QFontMetrics nm {numbers};
    const QColor faint = DosePaint::token(u"faint"_s);
    const QColor line = DosePaint::token(u"line"_s);

    const double top = niceMax(static_cast<double>(maxYValue()));
    QStringList labels;
    for (int i = 4; i >= 0; --i)
        labels << Utils::Misc::friendlyUnit(static_cast<qint64>(top * i / 4), true);
    int labelWidth = 0;
    for (const QString &l : labels)
        labelWidth = std::max(labelWidth, nm.horizontalAdvance(l));

    // Legend row on top, axis labels on the left and below.
    const int legendHeight = 28;
    const QRectF plot = QRectF(full).adjusted(labelWidth + 22, legendHeight + 10, -14, -(nm.height() + 12));
    if ((plot.width() < 40) || (plot.height() < 30))
        return;

    // Grid and Y labels.
    painter.setFont(numbers);
    for (int i = 0; i <= 4; ++i)
    {
        const qreal y = plot.top() + (plot.height() * i / 4.0);
        QPen pen {line, 1};
        if (i < 4)
            pen.setDashPattern({2, 4});
        painter.setPen(pen);
        painter.drawLine(QPointF(plot.left(), y), QPointF(plot.right(), y));
        painter.setPen(faint);
        painter.drawText(QRectF(full.left(), y - nm.height(), plot.left() - 10, 2 * nm.height()), Qt::AlignRight | Qt::AlignVCenter, labels[i]);
    }

    // X labels: the period on the left, now on the right.
    painter.setPen(faint);
    const QRectF xAxis {plot.left(), plot.bottom() + 6, plot.width(), qreal(nm.height())};
    painter.drawText(xAxis, Qt::AlignLeft | Qt::AlignVCenter, durationLabel(m_currentMaxDuration));
    painter.drawText(xAxis, Qt::AlignHCenter | Qt::AlignVCenter, durationLabel(m_currentMaxDuration / 2));
    painter.drawText(xAxis, Qt::AlignRight | Qt::AlignVCenter, tr("now"));

    // Curves.
    const DataCircularBuffer &queue = currentData();
    const milliseconds lastDuration {queue.empty() ? 0ms : queue.back().duration};
    const qreal span = std::max<qint64>(1, (m_currentMaxDuration - lastDuration).count());
    const qreal yScale = plot.height() / top;

    const auto colourOf = [this](const GraphID id) -> QColor
    {
        if (id == DOWN)
            return DosePaint::token(u"accent"_s);
        if (id == UP)
            return DosePaint::token(u"ok"_s);
        return m_properties[id].pen.color();
    };

    struct Legend { QString name; QColor colour; quint64 value; };
    QList<Legend> legend;
    painter.save();
    painter.setClipRect(plot.adjusted(-4, -4, 4, 1));
    // Draw upload first so download sits on top.
    for (const GraphID id : {UP, DOWN, PAYLOAD_UP, PAYLOAD_DOWN, OVERHEAD_UP, OVERHEAD_DOWN, DHT_UP, DHT_DOWN, TRACKER_UP, TRACKER_DOWN})
    {
        if (!m_properties[id].enable)
            continue;
        QList<QPointF> points;
        milliseconds duration {0};
        for (int i = static_cast<int>(queue.size()) - 1; i >= 0; --i)
        {
            points.prepend(QPointF(plot.right() - (duration.count() * plot.width() / span), plot.bottom() - (queue[i].data[id] * yScale)));
            duration += queue[i].duration;
            if (duration >= m_currentMaxDuration)
                break;
        }
        const QColor colour = colourOf(id);
        legend.append({m_properties[id].name, colour, queue.empty() ? 0 : queue.back().data[id]});
        if (points.isEmpty())
            continue;
        const QPainterPath path = smooth(points);
        const bool main = (id == UP) || (id == DOWN);
        if (main)
        {
            QPainterPath area = path;
            area.lineTo(points.last().x(), plot.bottom());
            area.lineTo(points.first().x(), plot.bottom());
            area.closeSubpath();
            QLinearGradient fill {plot.topLeft(), plot.bottomLeft()};
            QColor from = colour;
            from.setAlphaF(0.26);
            QColor to = colour;
            to.setAlphaF(0.0);
            fill.setColorAt(0, from);
            fill.setColorAt(1, to);
            painter.fillPath(area, fill);
        }
        painter.setPen(QPen(colour, main ? 1.8 : 1.2, Qt::SolidLine, Qt::RoundCap, Qt::RoundJoin));
        painter.setBrush(Qt::NoBrush);
        painter.drawPath(path);
        if (main)
        {
            painter.setPen(QPen(DosePaint::token(u"bg"_s), 2));
            painter.setBrush(colour);
            painter.drawEllipse(points.last(), 3.5, 3.5);
        }
    }
    painter.restore();

    // Legend: a colour square, the live value, the graph's name.
    qreal x = plot.left();
    const qreal y = full.top() + 8;
    QFont label = font();
    for (const Legend &item : legend)
    {
        painter.setPen(Qt::NoPen);
        painter.setBrush(item.colour);
        painter.drawRoundedRect(QRectF(x, y + 6, 10, 10), 3, 3);
        x += 16;
        const QString value = Utils::Misc::friendlyUnit(static_cast<qint64>(item.value), true);
        painter.setFont(numbers);
        painter.setPen(DosePaint::token(u"text"_s));
        painter.drawText(QPointF(x, y + 15), value);
        x += nm.horizontalAdvance(value) + 6;
        painter.setFont(label);
        painter.setPen(faint);
        painter.drawText(QPointF(x, y + 15), item.name);
        x += QFontMetrics(label).horizontalAdvance(item.name) + 22;
    }
}
