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

#include <QColor>
#include <QHash>
#include <QList>
#include <QString>

#include "gui/uithemesource.h"

// tTorrent's built-in look, on Kitab's model: a named theme (token file in
// src/gui/dose/theme/themes), an accent (a preset or any hex, adjusted in
// OKLCH lightness until it reaches 3:1 against the theme's background) and a
// density. The palette, state colours and stylesheet are all derived from
// those tokens, so no colour is written twice.
class DoseThemeSource final : public UIThemeSource
{
public:
    struct Choice
    {
        QString id;
        QString name;
    };

    // themeId must already be resolved ("system" is the caller's job).
    DoseThemeSource(const QString &themeId, const QString &accent, const QString &density);

    QColor getColor(const QString &colorId, ColorMode colorMode) const override;
    Path getIconPath(const QString &iconId, ColorMode colorMode) const override;
    QByteArray readStyleSheet() override;

    bool isDark() const;
    QColor token(const QString &name) const;
    // UI text size in px for the density.
    int fontPixelSize() const;

    // "system" first, then the light themes, then the dark ones.
    static QList<Choice> themes();
    // "theme" (the theme's own accent) first, then the presets.
    static QList<Choice> accentPresets();
    static QList<Choice> densities();
    // The accent before adjustment (swatches, the hex field).
    static QColor accentBase(const QString &accent, const QString &themeId);
    // Contrast of the unadjusted accent against the theme's background.
    static double accentContrast(const QString &accent, const QString &themeId);
    static QString resolveTheme(const QString &themeId, bool systemIsDark);
    // Older builds stored green/violet/ink; map them to today's presets.
    static QString migrateAccent(const QString &accent);

    static QString themeKey();    // Appearance/Theme, default "system"
    static QString accentKey();   // Appearance/Accent, default "theme"
    static QString densityKey();  // Appearance/Density, default "compact"
    static QString scaleKey();    // Appearance/InterfaceScale, default 1.0

private:
    QString m_themeId;
    QString m_density;
    bool m_dark = false;
    QHash<QString, QColor> m_tokens;
    QHash<QString, QColor> m_colors;
    FolderThemeSource m_icons;
};
