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

class QDialog;

#include "base/ttorrentversion.h"

namespace DoseAbout
{
    // Replaces the About dialog's content (DESIGN.md › About): logo, name and
    // version, links, and About / Credits / Licence / Build info tabs.
    // Credits name qBittorrent as where tTorrent began.
    void build(QDialog *dialog);
}
