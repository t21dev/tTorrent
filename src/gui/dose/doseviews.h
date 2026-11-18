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

class QBoxLayout;
class QTreeView;
class QWidget;

// Saved views: a "Views" section at the top of the sidebar. A view keeps the
// selected status, category, tag and tracker, the filter text and the sort
// column; clicking it brings them all back. Saved in tTorrent/Views.
namespace DoseViews
{
    // Adds the section as the first item of the sidebar's layout.
    void install(QBoxLayout *sidebarLayout, QWidget *sidebar, QTreeView *transferList);
}
