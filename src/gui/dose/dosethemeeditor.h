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
#include <QDialog>
#include <QHash>

class QComboBox;
class QLabel;
class QLineEdit;
class QToolButton;

// Create a theme (DESIGN.md › Theme editor): a name, a theme to start from,
// the thirteen colours as tiles (swatch, name, contrast, hex) in Surfaces,
// Text, and Accent and states, and a live preview. Saving writes
// <config>/themes/user-<name>.json and returns its id via savedThemeId().
class DoseThemeEditor final : public QDialog
{
    Q_OBJECT

public:
    explicit DoseThemeEditor(const QString &startFrom, QWidget *parent = nullptr);

    QString savedThemeId() const;

private:
    struct Tile
    {
        QToolButton *swatch = nullptr;
        QLineEdit *hex = nullptr;
        QLabel *ratio = nullptr;
    };

    void loadFrom(const QString &themeId);
    void setColour(const QString &token, const QColor &colour);
    void refresh();
    void save();
    void importTheme();
    void exportTheme();

    QLineEdit *m_name = nullptr;
    QComboBox *m_base = nullptr;
    QWidget *m_preview = nullptr;
    QHash<QString, Tile> m_tiles;
    QHash<QString, QColor> m_colours;
    bool m_dark = true;
    QString m_savedId;
};
