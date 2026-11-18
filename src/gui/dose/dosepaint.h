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

#pragma once

#include <QColor>
#include <QFont>

class QLabel;
class QWidget;
class QModelIndex;
class QPainter;
class QStyleOptionViewItem;

// Painting helpers for tTorrent's look in item views (DESIGN.md › Components).
// Every function is a no-op or returns a neutral value when a custom UI theme
// replaced the built-in look, so callers can use them unconditionally.
namespace DosePaint
{
    bool active();
    QColor token(const QString &name);

    // DM Mono, one pixel smaller than the view's font: sizes, speeds, counts.
    QFont numberFont(const QFont &base);

    // Right-aligned cells use the number font; zero values fade to `faint`.
    // Only the status column keeps its state colour; others use `text`.
    void styleCell(QStyleOptionViewItem *option, const QModelIndex &index, bool keepStateColour, bool isZero);

    // A 4 px pill on `track` with the percentage beside it, in `fill`.
    void progress(QPainter *painter, const QStyleOptionViewItem &option, qreal percent, const QString &text, const QColor &fill);

    // The 3 px accent bar on the selected row's first visible cell.
    void selectionBar(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index);

    // A label that qBittorrent colours red in its HTML becomes a quiet banner
    // in the theme's warning colour; a link in it stays clickable.
    void warningBanner(QLabel *label);

    // Select all / none buttons under `root` (Content tab, Add dialog)
    // become compact icon buttons with the text as their tooltip.
    void selectButtonsAsIcons(QWidget *root);
}
