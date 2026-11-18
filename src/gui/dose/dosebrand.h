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

class QTranslator;

namespace DoseBrand
{
    // Shows "tTorrent" wherever qBittorrent's user-visible text names the
    // app ("qBittorrent is minimized to tray", "Exiting qBittorrent", …), in
    // every language: a translator installed above qBittorrent's asks it for
    // the translation and swaps the name. qBittorrent's source strings and
    // translation files stay untouched, so they keep merging cleanly.
    void install(QTranslator *appTranslator);
}
