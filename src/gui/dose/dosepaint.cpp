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
#include <QAbstractItemView>
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
#include "base/settingsstorage.h"
#include "dosetheme.h"
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

namespace
{
    QString &cachedProgressStyle()
    {
        static QString style;
        return style;
    }
}

QString DosePaint::progressStyle()
{
    QString &style = cachedProgressStyle();
    if (style.isEmpty())
    {
        style = SettingsStorage::instance()->loadValue<QString>(u"Appearance/ProgressStyle"_s, u"line"_s);
        if ((style != u"bar") && (style != u"filled"))
            style = u"line"_s;
    }
    return style;
}

void DosePaint::setProgressStyle(const QString &style)
{
    SettingsStorage::instance()->storeValue(u"Appearance/ProgressStyle"_s, style);
    cachedProgressStyle().clear();
    for (QWidget *top : QApplication::topLevelWidgets())
    {
        for (QAbstractItemView *view : top->findChildren<QAbstractItemView *>())
            view->viewport()->update();
    }
}

void DosePaint::progress(QPainter *painter, const QStyleOptionViewItem &option, const qreal percent, const QString &text, const QColor &fill)
{
    if (paintsOwnRows(option.widget))
    {
        rowBackground(painter, option);
    }
    else
    {
        const QStyle *style = option.widget ? option.widget->style() : QApplication::style();
        style->drawPrimitive(QStyle::PE_PanelItemViewItem, &option, painter, option.widget);
    }

    const QFont font = numberFont(option.font);
    const QColor done = fill.isValid() ? fill : token(u"accent"_s);
    const qreal fraction = std::clamp(percent, 0.0, 100.0) / 100.0;
    const QString styleName = progressStyle();
    painter->save();
    painter->setRenderHint(QPainter::Antialiasing);
    painter->setPen(Qt::NoPen);
    painter->setFont(font);

    if (styleName == u"filled")
    {
        // The whole cell, rounded, the percentage centred; the text switches
        // colour where the fill passes under it.
        const QRectF box = QRectF(option.rect).adjusted(6, 4, -6, -4);
        const qreal radius = std::min<qreal>(6, box.height() / 2);
        painter->setBrush(token(u"track"_s));
        painter->drawRoundedRect(box, radius, radius);
        QRectF filled = box;
        filled.setWidth(box.width() * fraction);
        if (fraction > 0)
        {
            QPainterPath clip;
            clip.addRoundedRect(box, radius, radius);
            painter->setClipPath(clip);
            painter->setBrush(done);
            painter->drawRect(filled);
            painter->setClipping(false);
        }
        painter->setPen(token(u"text"_s));
        painter->setClipRect(QRectF(filled.right(), box.top(), box.width(), box.height()));
        painter->drawText(box, Qt::AlignCenter, text);
        painter->setPen(token(u"onAccent"_s));
        painter->setClipRect(filled);
        painter->drawText(box, Qt::AlignCenter, text);
        painter->restore();
        return;
    }

    const int textWidth = QFontMetrics(font).horizontalAdvance(u"100.0%"_s) + 2;
    const QRect cell = option.rect.adjusted(8, 0, -8, 0);
    const QRect textRect {cell.right() - textWidth + 1, cell.top(), textWidth, cell.height()};
    const int barHeight = (styleName == u"bar") ? 8 : 4;
    const QRectF bar {qreal(cell.left()), cell.center().y() - (barHeight / 2.0) + 1, qreal(std::max(0, cell.width() - textWidth - 8)), qreal(barHeight)};
    const qreal radius = barHeight / 2.0;
    painter->setBrush(token(u"track"_s));
    painter->drawRoundedRect(bar, radius, radius);
    if (fraction > 0)
    {
        QRectF part = bar;
        part.setWidth(std::max(qreal(barHeight), bar.width() * fraction));
        painter->setBrush(done);
        painter->drawRoundedRect(part, radius, radius);
    }
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

bool DosePaint::paintsOwnRows(const QWidget *view)
{
    return active() && view && view->inherits("TransferListWidget");
}

int DosePaint::rowPadding()
{
    return active() ? DoseThemeSource::rowPadding(UIThemeManager::instance()->lookDensity()) : 0;
}

void DosePaint::rowBackground(QPainter *painter, const QStyleOptionViewItem &option)
{
    QColor fill;
    if (option.state.testFlag(QStyle::State_Selected))
        fill = token(u"select"_s);
    else if (option.state.testFlag(QStyle::State_MouseOver))
        fill = token(u"hover"_s);
    if (fill.isValid())
        painter->fillRect(option.rect, fill);
    painter->fillRect(QRect(option.rect.left(), option.rect.bottom(), option.rect.width(), 1), token(u"rowRule"_s));
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
