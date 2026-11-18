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

#include "dosestyle.h"

#include <QApplication>
#include <QDialog>
#include <QFileDialog>
#include <QMenu>

#include "gui/titlebar/dosetitlebar.h"

DoseStyle::DoseStyle(const QString &baseStyle)
    : QProxyStyle(baseStyle)
{
}

int DoseStyle::styleHint(const StyleHint hint, const QStyleOption *option, const QWidget *widget, QStyleHintReturn *returnData) const
{
    if (hint == SH_UnderlineShortcut)
        return 0;
    return QProxyStyle::styleHint(hint, option, widget, returnData);
}

void DoseStyle::polish(QWidget *widget)
{
    QProxyStyle::polish(widget);
    if (!widget || !widget->isWindow() || widget->testAttribute(Qt::WA_WState_Created))
        return;

    // Menus and the list a combo box opens: round corners need a clear backing.
    const bool popup = (widget->windowType() == Qt::Popup) && (qobject_cast<QMenu *>(widget) || widget->inherits("QComboBoxPrivateContainer"));
    if (popup)
    {
        widget->setAttribute(Qt::WA_TranslucentBackground);
        widget->setWindowFlag(Qt::FramelessWindowHint);
        widget->setWindowFlag(Qt::NoDropShadowWindowHint);
        return;
    }

    if (auto *dialog = qobject_cast<QDialog *>(widget))
    {
        if (!qobject_cast<QFileDialog *>(dialog) && !dialog->property("doseFramed").toBool())
        {
            dialog->setProperty("doseFramed", true);
            DoseTitleBar::installDialog(dialog);
        }
    }
}

void DoseStyle::install()
{
    if (qobject_cast<DoseStyle *>(QApplication::style()))
        return;
    QApplication::setStyle(new DoseStyle(QApplication::style()->name()));
}
