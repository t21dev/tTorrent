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

#include <QString>

class QLabel;

// Status bar items as an icon and a value ("80.85 GiB" beside a drive icon)
// instead of words ("Free space: 80.85 GiB"). Right-click › Icons instead of
// words (Appearance/StatusBarIcons, on by default). The words stay in the
// tooltip.
namespace DoseStatus
{
    bool iconsOnly();
    void setIconsOnly(bool on);
    // Paints iconId at the label's left while icons are on.
    void decorate(QLabel *label, const QString &iconId);
    // "word value" with words on, "value" with icons on. word is the full
    // translated prefix, such as "Free space: ".
    QString text(const QString &word, const QString &value);
    // Moves a label's text to the current mode after a switch; tooltip is the
    // name shown on hover while icons are on.
    void relabel(QLabel *label, const QString &word, const QString &tooltip);
}
