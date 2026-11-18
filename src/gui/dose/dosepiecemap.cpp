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

// The piece map (DESIGN.md › General, the signature): the General tab's
// pieces bar drawn as a grid of small rounded squares. Downloaded pieces in
// the accent, pieces being downloaded in a lighter accent, missing ones on
// the track colour. When there are more pieces than squares, each square
// stands for a run of pieces: full if all are done, light if any are.

#include "gui/properties/downloadedpiecesbar.h"

#include <QPainter>

#include "base/global.h"
#include "dosepaint.h"

void DownloadedPiecesBar::paintEvent(QPaintEvent *event)
{
    if (!DosePaint::active())
    {
        PiecesBar::paintEvent(event);
        return;
    }

    QPainter painter {this};
    painter.setRenderHint(QPainter::Antialiasing);
    const QColor done = DosePaint::token(u"accent"_s);
    const QColor partial = DosePaint::token(u"accentLine"_s);
    const QColor missing = DosePaint::token(u"track"_s);

    const int cell = 9;
    const int gap = 2;
    const int columns = std::max(1, (width() + gap) / (cell + gap));
    const int rows = std::max(1, (height() + gap) / (cell + gap));
    const qsizetype pieces = m_pieces.size();
    if (pieces == 0)
    {
        // No torrent selected: an empty grid.
        painter.setPen(Qt::NoPen);
        painter.setBrush(missing);
        for (int r = 0; r < rows; ++r)
        {
            for (int c = 0; c < columns; ++c)
                painter.drawRoundedRect(QRectF(c * (cell + gap), r * (cell + gap), cell, cell), 2, 2);
        }
        return;
    }

    const qsizetype cells = std::min<qsizetype>(pieces, qsizetype(columns) * rows);
    painter.setPen(Qt::NoPen);
    for (qsizetype i = 0; i < cells; ++i)
    {
        const qsizetype from = (i * pieces) / cells;
        const qsizetype to = std::max(from + 1, ((i + 1) * pieces) / cells);
        qsizetype have = 0;
        bool active = false;
        for (qsizetype p = from; p < to; ++p)
        {
            if (m_pieces.testBit(p))
                ++have;
            else if (m_downloadedPieces.testBit(p))
                active = true;
        }
        const QColor colour = (have == (to - from)) ? done : ((have > 0) || active) ? partial : missing;
        painter.setBrush(colour);
        const int r = int(i / columns);
        const int c = int(i % columns);
        painter.drawRoundedRect(QRectF(c * (cell + gap), r * (cell + gap), cell, cell), 2, 2);
    }
}
