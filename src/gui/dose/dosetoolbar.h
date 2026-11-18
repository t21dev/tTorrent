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

class QAction;
class QComboBox;
class QLineEdit;
class QMenu;
class QTabWidget;
class QToolBar;

namespace DoseToolbar
{
    // Adds "Show buttons" (one checkbox per toolbar button) and "Reset
    // toolbar" to the toolbar's context menu. Hidden buttons leave the toolbar
    // but stay in the menus; separators never double up. Saved in
    // Appearance/ToolbarHidden. Call once the .ui toolbar actions exist;
    // widgets added to the toolbar later (the search field) stay after them.
    void install(QToolBar *toolBar, QMenu *contextMenu);

    // Toolbar icon size in px (16, 20 or 24; Appearance/ToolbarIconSize).
    void setIconSize(QToolBar *toolBar, int px);
    int iconSize();

    // Toolbar height: true is slim (the default), false is regular
    // (Appearance/ToolbarSlim). Applied as the toolbar's doseSlim property.
    void setSlim(QToolBar *toolBar, bool slim);
    bool isSlim();

    // The filter field: wider, its "Filter by" choice moved into a menu on
    // the field's search icon (the combo box stays, hidden, as the model).
    void polishFilter(QLineEdit *edit, QComboBox *columns);

    // The main tabs (Transfers, RSS, Log): the bar hides when Transfers is
    // the only tab; right-clicking it shows or hides the RSS and log tabs.
    void polishTabs(QTabWidget *tabs, QAction *rss, QAction *log);
}
