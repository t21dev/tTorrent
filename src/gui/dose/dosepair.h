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

class QMainWindow;
class QWidget;

// Pair a phone: a QR code that opens tTorrent's Web UI on a phone on the
// same network. Turns the Web UI on (with a username and password) first
// when it is off. Added to the Tools menu.
namespace DosePair
{
    void install(QMainWindow *window);
    void show(QWidget *parent);
}
