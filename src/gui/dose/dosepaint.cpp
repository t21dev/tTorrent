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

#include "dosepaint.h"

#include <QApplication>
#include <QHeaderView>
#include <QPainter>
#include <QPainterPath>
#include <QStyle>
#include <QStyleOptionViewItem>
#include <QTextDocumentFragment>
#include <QTreeView>
#include <QLabel>
#include <QPushButton>

#include "base/global.h"
#include "gui/uithememanager.h"

bool DosePaint::active()
{
    const UIThemeManager *manager = UIThemeManager::instance();
    return manager && manager->usesDoseTheme();
}

QColor DosePaint::token(const QString &name)
{
    return active() ? UIThemeManager::instance()->doseToken(name) : QColor();
}

QFont DosePaint::numberFont(const QFont &base)
{
    QFont font {u"DM Mono"_s};
    font.setStyleHint(QFont::Monospace, QFont::PreferAntialias);
    font.setHintingPreference(QFont::PreferNoHinting);
    const int px = (base.pixelSize() > 0) ? base.pixelSize() : qRound(base.pointSizeF() * 96.0 / 72.0);
    font.setPixelSize(std::max(9, px - 1));
    return font;
}

void DosePaint::styleCell(QStyleOptionViewItem *option, const QModelIndex &index, const bool keepStateColour, const bool isZero)
{
    if (!active())
        return;
    Q_UNUSED(index);
    if (option->displayAlignment.testFlag(Qt::AlignRight))
        option->font = numberFont(option->font);
    QColor colour = keepStateColour ? QColor() : token(u"text"_s);
    if (isZero)
        colour = token(u"faint"_s);
    if (colour.isValid())
    {
        option->palette.setColor(QPalette::Text, colour);
        option->palette.setColor(QPalette::HighlightedText, colour);
    }
}

void DosePaint::progress(QPainter *painter, const QStyleOptionViewItem &option, const qreal percent, const QString &text, const QColor &fill)
{
    const QStyle *style = option.widget ? option.widget->style() : QApplication::style();
    style->drawPrimitive(QStyle::PE_PanelItemViewItem, &option, painter, option.widget);

    const QFont font = numberFont(option.font);
    const int textWidth = QFontMetrics(font).horizontalAdvance(u"100.0%"_s) + 2;
    const QRect cell = option.rect.adjusted(8, 0, -8, 0);
    const QRect textRect {cell.right() - textWidth + 1, cell.top(), textWidth, cell.height()};
    const int barHeight = 4;
    const QRectF bar {qreal(cell.left()), cell.center().y() - (barHeight / 2.0) + 1, qreal(std::max(0, cell.width() - textWidth - 8)), qreal(barHeight)};

    painter->save();
    painter->setRenderHint(QPainter::Antialiasing);
    painter->setPen(Qt::NoPen);
    painter->setBrush(token(u"track"_s));
    painter->drawRoundedRect(bar, 2, 2);
    if (percent > 0)
    {
        QRectF done = bar;
        done.setWidth(std::max(qreal(barHeight), bar.width() * std::clamp(percent, 0.0, 100.0) / 100.0));
        painter->setBrush(fill.isValid() ? fill : token(u"accent"_s));
        painter->drawRoundedRect(done, 2, 2);
    }
    painter->setFont(font);
    painter->setPen(token(u"muted"_s));
    painter->drawText(textRect, Qt::AlignRight | Qt::AlignVCenter, text);
    painter->restore();
}

void DosePaint::warningBanner(QLabel *label)
{
    if (!label || !active())
        return;
    const QString plain = QTextDocumentFragment::fromHtml(label->text()).toPlainText().trimmed();
    const QColor warn = token(u"warn"_s);
    const bool link = label->text().contains(u"<a "_s);
    label->setText(link ? u"<a href=\"#\" style=\"color:%1; text-decoration:none;\">%2</a>"_s.arg(warn.name(), plain.toHtmlEscaped()) : plain);
    QColor wash = warn;
    wash.setAlphaF(0.12f);
    label->setStyleSheet(u"QLabel { color: %1; background: rgba(%2, %3, %4, %5); border: 1px solid rgba(%2, %3, %4, 0.35); border-radius: 8px; padding: 6px 10px; font-style: normal; }"_s
        .arg(warn.name()).arg(wash.red()).arg(wash.green()).arg(wash.blue()).arg(wash.alphaF()));
}

void DosePaint::selectButtonsAsIcons(QWidget *root)
{
    if (!root || !active())
        return;
    for (QPushButton *button : root->findChildren<QPushButton *>())
    {
        const QString name = button->objectName();
        const bool all = (name == u"selectAllButton") || (name == u"buttonSelectAll");
        const bool none = (name == u"selectNoneButton") || (name == u"buttonSelectNone");
        if ((!all && !none) || button->property("doseIconButton").toBool())
            continue;
        button->setProperty("doseIconButton", true);
        button->setToolTip(button->text().remove(u'&'));
        button->setText(QString());
        button->setIcon(UIThemeManager::instance()->getIcon(all ? u"dose-select-all"_s : u"dose-select-none"_s));
        button->setIconSize({16, 16});
        button->setFixedSize(30, 28);
        button->style()->unpolish(button);
        button->style()->polish(button);
    }
}

void DosePaint::selectionBar(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index)
{
    if (!active() || !option.state.testFlag(QStyle::State_Selected))
        return;
    const auto *view = qobject_cast<const QTreeView *>(option.widget);
    if (!view || (view->header()->logicalIndex(0) != index.column()))
        return;
    painter->fillRect(QRect(option.rect.left(), option.rect.top(), 3, option.rect.height()), token(u"accent"_s));
}
