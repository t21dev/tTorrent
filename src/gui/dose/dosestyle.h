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

#include <QProxyStyle>

// tTorrent's layer over the platform style while the built-in look is on:
// - menus and dropdown lists get transparent windows, so the stylesheet's
//   rounded corners show instead of a square backing;
// - every dialog gets the custom title bar (DoseTitleBar::installDialog);
// - mnemonic underlines stay hidden (Alt still works), as on Windows.
class DoseStyle final : public QProxyStyle
{
    Q_OBJECT

public:
    explicit DoseStyle(const QString &baseStyle);

    int styleHint(StyleHint hint, const QStyleOption *option, const QWidget *widget, QStyleHintReturn *returnData) const override;
    void polish(QWidget *widget) override;
    using QProxyStyle::polish;

    // Wraps the application's current style once; safe to call again.
    static void install();
};
