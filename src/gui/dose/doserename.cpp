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

#include "doserename.h"

#include <algorithm>

#include <QCoreApplication>
#include <QInputDialog>
#include <QMenu>
#include <QMessageBox>

#include "base/bittorrent/categoryoptions.h"
#include "base/bittorrent/session.h"
#include "base/bittorrent/torrent.h"
#include "base/global.h"
#include "base/logger.h"
#include "base/tag.h"
#include "gui/uithememanager.h"

namespace
{
    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseRename", text);
    }

    void ask(QWidget *parent, const QString &title, const QString &current, const std::function<QString (const QString &)> &apply)
    {
        bool ok = false;
        const QString name = QInputDialog::getText(parent, title, tr("New name:"), QLineEdit::Normal, current, &ok).trimmed();
        if (!ok || name.isEmpty() || (name == current))
            return;
        if (const QString error = apply(name); !error.isEmpty())
            QMessageBox::warning(parent, title, error);
    }
}

QString DoseRename::renameCategory(const QString &from, const QString &to)
{
    BitTorrent::Session *session = BitTorrent::Session::instance();
    const QStringList existing = session->categories();
    if (!existing.contains(from))
        return tr("The category \"%1\" no longer exists.").arg(from);
    if (!BitTorrent::Session::isValidCategoryName(to))
        return tr("\"%1\" isn't a valid category name.").arg(to);
    if (existing.contains(to))
        return tr("A category named \"%1\" already exists.").arg(to);
    if (to.startsWith(from + u'/'))
        return tr("A category can't be moved inside itself.");

    // The category and its subcategories ("Films" and "Films/4K"), parents first.
    QStringList affected;
    for (const QString &name : existing)
    {
        if ((name == from) || name.startsWith(from + u'/'))
            affected.append(name);
    }
    std::sort(affected.begin(), affected.end(), [](const QString &a, const QString &b) { return a.size() < b.size(); });
    QHash<QString, QString> renamed;
    for (const QString &name : affected)
    {
        const QString target = to + name.mid(from.size());
        if (!session->categories().contains(target) && !session->addCategory(target, session->categoryOptions(name)))
            return tr("Couldn't create the category \"%1\".").arg(target);
        renamed.insert(name, target);
    }
    int moved = 0;
    for (BitTorrent::Torrent *torrent : session->torrents())
    {
        if (const auto it = renamed.constFind(torrent->category()); it != renamed.cend())
            moved += torrent->setCategory(it.value()) ? 1 : 0;
    }
    // Deepest first, so removing a parent never takes a child with it.
    std::reverse(affected.begin(), affected.end());
    for (const QString &name : affected)
        session->removeCategory(name);
    LogMsg(tr("Renamed category \"%1\" to \"%2\" (%3 torrents).").arg(from, to, QString::number(moved)));
    return {};
}

QString DoseRename::renameTag(const Tag &from, const QString &to)
{
    BitTorrent::Session *session = BitTorrent::Session::instance();
    const Tag target {to};
    if (!target.isValid())
        return tr("\"%1\" isn't a valid tag.").arg(to);
    if (session->hasTag(target))
        return tr("A tag named \"%1\" already exists.").arg(to);
    if (!session->addTag(target))
        return tr("Couldn't create the tag \"%1\".").arg(to);
    int moved = 0;
    for (BitTorrent::Torrent *torrent : session->torrents())
    {
        if (torrent->hasTag(from))
        {
            torrent->addTag(target);
            torrent->removeTag(from);
            ++moved;
        }
    }
    session->removeTag(from);
    LogMsg(tr("Renamed tag \"%1\" to \"%2\" (%3 torrents).").arg(from.toString(), to, QString::number(moved)));
    return {};
}

void DoseRename::addToCategoryMenu(QMenu *menu, QWidget *parent, const QString &category)
{
    if (!menu || category.isEmpty())
        return;
    menu->addAction(UIThemeManager::instance()->getIcon(u"edit-rename"_s), tr("Rename category..."), parent, [parent, category]
    {
        ask(parent, tr("Rename category"), category, [category](const QString &to) { return renameCategory(category, to); });
    });
}

void DoseRename::addToTagMenu(QMenu *menu, QWidget *parent, const Tag &tag)
{
    if (!menu || !tag.isValid())
        return;
    menu->addAction(UIThemeManager::instance()->getIcon(u"edit-rename"_s), tr("Rename tag..."), parent, [parent, tag]
    {
        ask(parent, tr("Rename tag"), tag.toString(), [tag](const QString &to) { return renameTag(tag, to); });
    });
}
