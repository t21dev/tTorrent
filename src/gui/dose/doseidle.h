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

class QAbstractItemModel;
class QWidget;

// Saves CPU while nobody is looking: the transfer list stops redrawing,
// re-sorting and re-filtering while its window is hidden or minimized,
// and catches up with one refresh when the window comes back.
namespace DoseIdle
{
    // Sets the model's "dosePaused" property while `view`'s window is
    // hidden or minimized. The model skips row updates then and sets
    // "dosePending"; on show it gets one full dataChanged.
    void pauseWhileHidden(QAbstractItemModel *model, QWidget *view);

    // True while `widget`'s window is hidden or minimized.
    bool windowHidden(const QWidget *widget);
}
