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

class QMenu;
class QWidget;
class Tag;

// Rename categories and tags (qBittorrent issue #10614). A category keeps its
// save paths and subcategories; every torrent in it moves to the new name
// (no files move). A tag moves to its new name on every torrent that has it.
namespace DoseRename
{
    void addToCategoryMenu(QMenu *menu, QWidget *parent, const QString &category);
    void addToTagMenu(QMenu *menu, QWidget *parent, const Tag &tag);

    // Return an error message, or an empty string on success.
    QString renameCategory(const QString &from, const QString &to);
    QString renameTag(const Tag &from, const QString &to);
}
