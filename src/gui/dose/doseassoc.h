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

// Makes tTorrent a choice for .torrent files and magnet links on Windows.
// Windows 11 only lets the user pick the default, so tTorrent registers
// itself for the current user (no admin rights needed) and opens its page in
// Settings › Default apps, where one click sets both.
namespace DoseAssoc
{
    // Writes tTorrent's file type, magnet handler and capabilities under
    // HKEY_CURRENT_USER, pointing at this exe.
    void registerForUser();
    // Adds the magnet scheme key if this exe is registered but the key is
    // missing (installs made before it existed). Cheap: a few registry reads.
    void repairMagnetScheme();
    // registerForUser(), then opens tTorrent's page in Default apps.
    void registerAndOpenSettings();
}
