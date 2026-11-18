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

class QWidget;

// Hover and press that fade instead of snapping (ttorrent/DESIGN.md › Motion).
// Buttons keep their size, padding and text colour from the stylesheet; their
// background and border are painted here, under the widget, from the same
// tokens, with the hover amount easing over 120 ms. Nothing runs while the
// pointer is still. Windows' "Animation effects" setting off makes it instant.
namespace DoseMotion
{
    // From DoseStyle::polish: takes over the background of buttons it knows.
    void attach(QWidget *widget);
}
