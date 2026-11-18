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

#include "dosesidebar.h"

#include <QAbstractItemView>
#include <QCheckBox>
#include <QPainter>
#include <QRegularExpression>
#include <QStyledItemDelegate>

#include "dosepaint.h"

namespace
{
    // "Downloading (4)" → name "Downloading", count "4".
    bool split(const QString &text, QString *name, QString *count)
    {
        static const QRegularExpression pattern {QStringLiteral(R"(^(.*\S)\s*\((\d+)\)$)")};
        const QRegularExpressionMatch m = pattern.match(text);
        if (!m.hasMatch())
            return false;
        *name = m.captured(1);
        *count = m.captured(2);
        return true;
    }

    class CountDelegate final : public QStyledItemDelegate
    {
    public:
        using QStyledItemDelegate::QStyledItemDelegate;

        void paint(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index) const override
        {
            QStyleOptionViewItem opt {option};
            initStyleOption(&opt, index);
            QString name, count;
            const bool hasCount = split(opt.text, &name, &count);
            if (hasCount)
                opt.text = name;
            const QWidget *widget = option.widget;
            (widget ? widget->style() : nullptr)->drawControl(QStyle::CE_ItemViewItem, &opt, painter, widget);
            if (!hasCount)
                return;
            painter->save();
            painter->setFont(DosePaint::numberFont(option.font));
            painter->setPen(DosePaint::token(QStringLiteral("faint")));
            painter->drawText(option.rect.adjusted(0, 0, -10, 0), Qt::AlignRight | Qt::AlignVCenter, count);
            painter->restore();
        }
    };
}

void DoseSidebar::polishSection(QCheckBox *heading, QWidget *filterView)
{
    if (!DosePaint::active())
        return;
    if (heading)
    {
        QFont font = heading->font();
        font.setCapitalization(QFont::MixedCase);
        font.setBold(false);
        font.setWeight(QFont::DemiBold);
        heading->setFont(font);
        heading->setObjectName(QStringLiteral("doseSideHeading"));
    }
    if (auto *view = qobject_cast<QAbstractItemView *>(filterView))
    {
        view->setItemDelegate(new CountDelegate(view));
        view->setObjectName(QStringLiteral("doseSideList"));
        view->setFrameShape(QFrame::NoFrame);
    }
}
