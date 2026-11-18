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
#include <QAbstractItemView>
#include <QDialog>
#include <QFileDialog>
#include <QMenu>
#include <QPropertyAnimation>
#include <QPushButton>
#include <QTimer>

#include "dosemotion.h"
#include "dosepaint.h"
#include "gui/titlebar/dosetitlebar.h"
#include "gui/uithememanager.h"

namespace
{
    // Fades a window in when it shows: 120 ms for menus, lists and tooltips,
    // 180 ms for dialogs (DESIGN.md › Motion). Skipped when the system turns
    // animations off (Qt reads that into UI_AnimateMenu on Windows).
    class FadeIn final : public QObject
    {
    public:
        using QObject::QObject;

        static FadeIn *instance()
        {
            static auto *filter = new FadeIn(qApp);
            return filter;
        }

        // Gives a dialog the custom title bar once it has a layout (dialogs
        // are often polished before their layout exists).
        static void frame(QDialog *dialog)
        {
            if (!dialog->layout() || dialog->property("doseFramed").toBool())
                return;
            dialog->setProperty("doseFramed", true);
            DoseTitleBar::installDialog(dialog);
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if (auto *dialog = qobject_cast<QDialog *>(watched);
                dialog && ((event->type() == QEvent::LayoutRequest) || (event->type() == QEvent::Show) || (event->type() == QEvent::Polish)))
            {
                frame(dialog);
            }
            if ((event->type() == QEvent::Show) && QApplication::isEffectEnabled(Qt::UI_AnimateMenu))
            {
                auto *window = static_cast<QWidget *>(watched);
                const bool dialog = qobject_cast<QDialog *>(window);
                auto *animation = new QPropertyAnimation(window, "windowOpacity", window);
                animation->setStartValue(0.0);
                animation->setEndValue(1.0);
                animation->setDuration(dialog ? 180 : 120);
                animation->setEasingCurve(QEasingCurve::OutCubic);
                window->setWindowOpacity(0.0);
                animation->start(QAbstractAnimation::DeleteWhenStopped);
            }
            return QObject::eventFilter(watched, event);
        }
    };
}

DoseStyle::DoseStyle(const QString &baseStyle)
    // Always Fusion underneath: Qt switches to its Windows 11 style when the
    // colour scheme is light, whose list rows grew taller in Qt 6.12, so light
    // and dark themes would differ in size. The look is designed on Fusion.
    : QProxyStyle(u"Fusion"_s)
{
    Q_UNUSED(baseStyle);
}

int DoseStyle::styleHint(const StyleHint hint, const QStyleOption *option, const QWidget *widget, QStyleHintReturn *returnData) const
{
    if (hint == SH_UnderlineShortcut)
        return 0;
    return QProxyStyle::styleHint(hint, option, widget, returnData);
}

int DoseStyle::pixelMetric(const PixelMetric metric, const QStyleOption *option, const QWidget *widget) const
{
    const UIThemeManager *manager = UIThemeManager::instance();
    if (!manager || !manager->usesDoseTheme())
        return QProxyStyle::pixelMetric(metric, option, widget);
    // dense, compact, standard
    const QString density = manager->lookDensity();
    const int level = (density == u"dense") ? 0 : ((density == u"standard") ? 2 : 1);
    const auto pick = [level](const int dense, const int compact, const int standard)
    {
        return (level == 0) ? dense : ((level == 1) ? compact : standard);
    };
    switch (metric)
    {
    case PM_LayoutLeftMargin:
    case PM_LayoutRightMargin:
    case PM_LayoutTopMargin:
    case PM_LayoutBottomMargin:
        return pick(6, 9, 12);
    case PM_LayoutHorizontalSpacing:
    case PM_LayoutVerticalSpacing:
        return pick(4, 6, 8);
    case PM_SmallIconSize:
    case PM_ButtonIconSize:
    case PM_TabBarIconSize:
    case PM_ListViewIconSize:
        return pick(14, 14, 16);
    case PM_LargeIconSize:
        return pick(18, 20, 24);
    default:
        return QProxyStyle::pixelMetric(metric, option, widget);
    }
}

void DoseStyle::polish(QWidget *widget)
{
    QProxyStyle::polish(widget);
    if (!widget)
        return;
    // Select all / none: icon buttons once the form around them is built
    // (qBittorrent sets their text after polish, so wait one turn).
    if (auto *button = qobject_cast<QPushButton *>(widget);
        button && (button->objectName().startsWith(u"select") || button->objectName().startsWith(u"buttonSelect")))
    {
        QTimer::singleShot(0, button, [button] { DosePaint::selectButtonsAsIcons(button->parentWidget()); });
    }
    DoseMotion::attach(widget); // buttons: background painted with a fading hover
    // Lists styled row by row through the stylesheet; the transfer list
    // paints its own rows (DosePaint::paintsOwnRows).
    if (auto *view = qobject_cast<QAbstractItemView *>(widget); view && !view->inherits("TransferListWidget"))
        view->setProperty("doseRows", true);
    if (!widget->isWindow())
        return;

    // Every dialog gets the custom title bar (never the system one), as soon
    // as it has a layout; the file picker stays native.
    if (auto *dialog = qobject_cast<QDialog *>(widget); dialog && !qobject_cast<QFileDialog *>(dialog))
    {
        dialog->installEventFilter(FadeIn::instance());
        FadeIn::frame(dialog);
        return;
    }
    if (widget->testAttribute(Qt::WA_WState_Created))
        return;

    // Menus and the list a combo box opens: round corners need a clear backing.
    const bool popup = (widget->windowType() == Qt::Popup) && (qobject_cast<QMenu *>(widget) || widget->inherits("QComboBoxPrivateContainer"));
    if (popup)
    {
        widget->setAttribute(Qt::WA_TranslucentBackground);
        widget->setWindowFlag(Qt::FramelessWindowHint);
        widget->setWindowFlag(Qt::NoDropShadowWindowHint);
        widget->installEventFilter(FadeIn::instance());
        if (auto *menu = qobject_cast<QMenu *>(widget); menu && (menu->actions().size() > 16))
            menu->setProperty("doseDense", true);
        return;
    }
    if (widget->windowType() == Qt::ToolTip)
    {
        widget->installEventFilter(FadeIn::instance());
        return;
    }

}

QIcon DoseStyle::standardIcon(const StandardPixmap icon, const QStyleOption *option, const QWidget *widget) const
{
    const char *id = nullptr;
    switch (icon)
    {
    case SP_DirIcon:
    case SP_DirOpenIcon:
    case SP_DirClosedIcon:
    case SP_DirLinkIcon:
    case SP_DirLinkOpenIcon:
    case SP_DialogOpenButton:
    case SP_FileDialogStart:
        id = "directory";
        break;
    case SP_FileIcon:
    case SP_FileLinkIcon:
        id = "fileicon";
        break;
    case SP_MessageBoxWarning:
        id = "dialog-warning";
        break;
    case SP_MessageBoxCritical:
        id = "error";
        break;
    case SP_MessageBoxInformation:
        id = "help-about";
        break;
    case SP_MessageBoxQuestion:
        id = "help-contents";
        break;
    case SP_BrowserReload:
        id = "view-refresh";
        break;
    case SP_TrashIcon:
        id = "list-remove";
        break;
    default:
        break;
    }
    if (id && UIThemeManager::instance() && UIThemeManager::instance()->usesDoseTheme())
        return UIThemeManager::instance()->getIcon(QString::fromLatin1(id));
    return QProxyStyle::standardIcon(icon, option, widget);
}

void DoseStyle::drawPrimitive(const PrimitiveElement element, const QStyleOption *option, QPainter *painter, const QWidget *widget) const
{
    if ((element == PE_FrameFocusRect) && qobject_cast<const QAbstractItemView *>(widget))
        return;
    QProxyStyle::drawPrimitive(element, option, painter, widget);
}

void DoseStyle::install()
{
    if (qobject_cast<DoseStyle *>(QApplication::style()))
        return;
    QApplication::setStyle(new DoseStyle(QApplication::style()->name()));
}
