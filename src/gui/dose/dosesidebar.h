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

class QCheckBox;
class QWidget;

namespace DoseSidebar
{
    // One section of the filter sidebar (Status, Categories, Tags, Trackers):
    // a sentence-case heading in `faint` instead of bold capitals, and items
    // drawn as "Name ........ 12" with the count right-aligned in DM Mono
    // (qBittorrent writes "Name (12)"; the text is split when drawn).
    void polishSection(QCheckBox *heading, QWidget *filterView);
}
