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

class QComboBox;
class QLineEdit;
class QMenu;
class QToolBar;

namespace DoseToolbar
{
    // Adds "Show buttons" (one checkbox per toolbar button) and "Reset
    // toolbar" to the toolbar's context menu. Hidden buttons leave the toolbar
    // but stay in the menus; separators never double up. Saved in
    // Appearance/ToolbarHidden. Call once the .ui toolbar actions exist;
    // widgets added to the toolbar later (the search field) stay after them.
    void install(QToolBar *toolBar, QMenu *contextMenu);

    // The filter field: wider, its "Filter by" choice moved into a menu on
    // the field's search icon (the combo box stays, hidden, as the model).
    void polishFilter(QLineEdit *edit, QComboBox *columns);
}
