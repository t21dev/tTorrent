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

#include "dosemotion.h"

#include <QEvent>
#include <QPainter>
#include <QPainterPath>
#include <QPushButton>
#include <QStatusBar>
#include <QToolBar>
#include <QToolButton>
#include <QVariantAnimation>

#include "base/global.h"
#include "dosepaint.h"
#include "dosetheme.h"
#include "gui/uithememanager.h"

namespace
{
    // What a button looks like at rest, hovered and pressed. Kinds match the
    // stylesheet's button rules, which now leave the background to us.
    enum class Kind
    {
        Raised,     // QPushButton: raised fill, line border, darker bottom edge
        Primary,    // the default button and Add torrent: accent fill
        Quiet,      // transparent until hovered: toolbar, flat, status bar, search icon
        IconButton, // transparent; hover adds a fill and a line border
    };

    const char KIND[] = "doseMotionKind";
    const char AMOUNT[] = "doseMotionHover";

    QColor token(const char *name)
    {
        return UIThemeManager::instance()->doseToken(QString::fromLatin1(name));
    }

    QColor mix(const QColor &a, const QColor &b, const qreal t)
    {
        if (!a.isValid() || (a.alpha() == 0))
        {
            QColor c = b;
            c.setAlphaF(b.alphaF() * t);
            return c;
        }
        return QColor::fromRgbF(a.redF() + ((b.redF() - a.redF()) * t), a.greenF() + ((b.greenF() - a.greenF()) * t)
            , a.blueF() + ((b.blueF() - a.blueF()) * t), a.alphaF() + ((b.alphaF() - a.alphaF()) * t));
    }

    int radius()
    {
        return DoseThemeSource::controlRadius(UIThemeManager::instance()->lookDensity());
    }

    template <typename T>
    bool inside(const QWidget *widget)
    {
        for (const QWidget *p = widget->parentWidget(); p; p = p->parentWidget())
        {
            if (qobject_cast<const T *>(p))
                return true;
        }
        return false;
    }

    bool takeOver(QWidget *widget, Kind *kind)
    {
        // A widget with its own style sheet (accent swatches) draws itself.
        if (!widget->styleSheet().isEmpty())
            return false;
        if (auto *button = qobject_cast<QPushButton *>(widget))
        {
            // Window buttons (minimize, maximize, close) keep the title bar's own look.
            if (button->inherits("QWK::WindowButton") || button->property("system-button").isValid())
                return false;
            if (button->property("doseTab").toBool())
                return false; // tabs only change text colour and their underline
            if (button->property("doseIconButton").toBool())
                *kind = Kind::IconButton;
            else if (button->isFlat() || inside<QStatusBar>(button))
                *kind = Kind::Quiet;
            else
                *kind = button->isDefault() ? Kind::Primary : Kind::Raised;
            return true;
        }
        if (auto *tool = qobject_cast<QToolButton *>(widget))
        {
            if (tool->objectName() == u"dosePrimary")
                *kind = Kind::Primary;
            else if ((tool->objectName() == u"doseSearchIcon") || inside<QToolBar>(tool))
                *kind = Kind::Quiet;
            else
                return false; // other tool buttons (title bar, cards) keep their own look
            return true;
        }
        return false;
    }

    void paintBackground(QWidget *widget, const Kind kind, const qreal hover)
    {
        auto *button = qobject_cast<QAbstractButton *>(widget);
        const bool enabled = widget->isEnabled();
        const bool down = button && button->isDown();
        const bool checked = button && button->isChecked();
        // Leave out the margins the stylesheet gives some buttons.
        QMarginsF margin;
        if (widget->objectName() == u"dosePrimary")
            margin = {0, 0, 6, 0};
        else if ((widget->objectName() == u"trackerUpButton") || (widget->objectName() == u"trackerDownButton"))
            margin = {4, 2, 4, 2};
        const QRectF r = QRectF(widget->rect()).marginsRemoved(margin).adjusted(0.5, 0.5, -0.5, -0.5);
        // Corner radii as the stylesheet set them for each kind.
        const bool searchIcon = (widget->objectName() == u"doseSearchIcon");
        const qreal rad = (kind == Kind::IconButton) ? 7 : (searchIcon ? 6 : radius());
        QPainter p {widget};
        p.setRenderHint(QPainter::Antialiasing);

        QColor fill, border, bottom;
        switch (kind)
        {
        case Kind::Raised:
            fill = !enabled ? token("panel") : (down ? token("well") : mix(token("raised"), token("hover"), hover));
            border = !enabled ? token("line") : mix(token("line"), token("ring"), down ? 0 : hover);
            bottom = !enabled ? token("line") : (down ? token("line") : mix(token("edgeLo"), token("ring"), hover));
            break;
        case Kind::Primary:
            fill = !enabled ? token("panel") : (down ? token("accentBottom") : mix(token("accent"), token("accentHover"), hover));
            border = !enabled ? token("line") : fill;
            bottom = border;
            break;
        case Kind::Quiet:
            if (!enabled)
                return;
            fill = down ? token("panel") : (checked ? token("select") : mix(QColor(Qt::transparent), token("hover"), hover));
            break;
        case Kind::IconButton:
            if (!enabled)
                return;
            fill = mix(QColor(Qt::transparent), token("hover"), down ? 1.0 : hover);
            border = mix(QColor(Qt::transparent), token("line"), down ? 1.0 : hover);
            bottom = border;
            break;
        }

        QPainterPath outer;
        outer.addRoundedRect(r, rad, rad);
        if (border.isValid() && (border.alpha() > 0))
        {
            // A 1 px border whose bottom edge can differ, like the stylesheet's.
            p.fillPath(outer, border);
            if (bottom != border)
            {
                p.save();
                p.setClipRect(QRectF(r.left(), r.bottom() - 1.5, r.width(), 2));
                p.fillPath(outer, bottom);
                p.restore();
            }
            QPainterPath inner;
            inner.addRoundedRect(r.adjusted(1, 1, -1, -1), rad - 1, rad - 1);
            p.setCompositionMode(QPainter::CompositionMode_Source);
            p.fillPath(inner, fill.isValid() ? fill : QColor(Qt::transparent));
            p.setCompositionMode(QPainter::CompositionMode_SourceOver);
        }
        else if (fill.isValid() && (fill.alpha() > 0))
        {
            p.fillPath(outer, fill);
        }
    }

    class Animator final : public QObject
    {
    public:
        static Animator *instance()
        {
            static auto *animator = new Animator;
            return animator;
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            auto *widget = static_cast<QWidget *>(watched);
            switch (event->type())
            {
            case QEvent::Enter:
            case QEvent::HoverEnter:
                fade(widget, 1.0);
                break;
            case QEvent::Leave:
            case QEvent::HoverLeave:
                fade(widget, 0.0);
                break;
            case QEvent::Paint:
                if (DosePaint::active())
                {
                    Kind kind;
                    if (takeOver(widget, &kind))
                        paintBackground(widget, kind, widget->property(AMOUNT).toReal());
                }
                break;
            default:
                break;
            }
            return QObject::eventFilter(watched, event);
        }

    private:
        void fade(QWidget *widget, const qreal to)
        {
            const qreal from = widget->property(AMOUNT).toReal();
            if (qFuzzyCompare(from + 1, to + 1))
                return;
            if (!DosePaint::animationsEnabled())
            {
                widget->setProperty(AMOUNT, to);
                widget->update();
                return;
            }
            auto *animation = widget->findChild<QVariantAnimation *>(u"doseMotion"_s, Qt::FindDirectChildrenOnly);
            if (!animation)
            {
                animation = new QVariantAnimation(widget);
                animation->setObjectName(u"doseMotion"_s);
                animation->setEasingCurve(QEasingCurve::OutCubic);
                QObject::connect(animation, &QVariantAnimation::valueChanged, widget, [widget](const QVariant &value)
                {
                    widget->setProperty(AMOUNT, value.toReal());
                    widget->update();
                });
            }
            animation->stop();
            animation->setStartValue(from);
            animation->setEndValue(to);
            // Out is a touch slower than in, so leaving doesn't flicker.
            animation->setDuration((to > from) ? 120 : 180);
            animation->start();
        }
    };
}

void DoseMotion::attach(QWidget *widget)
{
    Kind kind;
    if (!widget || widget->property(KIND).isValid() || !takeOver(widget, &kind))
        return;
    widget->setProperty(KIND, int(kind));
    widget->setProperty(AMOUNT, 0.0);
    widget->setAttribute(Qt::WA_Hover);
    widget->installEventFilter(Animator::instance());
}
