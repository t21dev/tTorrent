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

#include "dosestatus.h"

#include <QEvent>
#include <QLabel>
#include <QPainter>

#include "base/global.h"
#include "base/settingsstorage.h"
#include "dosepaint.h"
#include "gui/uithememanager.h"

namespace
{
    const QString KEY = u"Appearance/StatusBarIcons"_s;
    const char ICON[] = "doseStatusIcon";
    const int GAP = 5;
    const int PAD = 8; // the space words had before the first item

    int iconSize(const QLabel *label)
    {
        return std::min(16, label->fontMetrics().height());
    }

    QMargins baseMargins(QLabel *label);

    class Decorator final : public QObject
    {
    public:
        using QObject::QObject;

        static Decorator *instance()
        {
            static auto *decorator = new Decorator;
            return decorator;
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if ((event->type() == QEvent::Paint) && DoseStatus::iconsOnly())
            {
                auto *label = static_cast<QLabel *>(watched);
                const int size = iconSize(label);
                const QIcon icon = UIThemeManager::instance()->getIcon(label->property(ICON).toString());
                QPainter painter {label};
                const QRect r {PAD, (label->height() - size) / 2, size, size};
                icon.paint(&painter, r, Qt::AlignCenter, label->isEnabled() ? QIcon::Normal : QIcon::Disabled);
            }
            return QObject::eventFilter(watched, event);
        }
    };

    // The stylesheet's own padding, kept on the label the first time.
    QMargins baseMargins(QLabel *label)
    {
        if (!label->property("doseStatusBase").isValid())
            label->setProperty("doseStatusBase", QVariant::fromValue(label->contentsMargins()));
        return label->property("doseStatusBase").value<QMargins>();
    }

    void applyMargin(QLabel *label)
    {
        QMargins m = baseMargins(label);
        if (DoseStatus::iconsOnly())
            m.setLeft(PAD + iconSize(label) + GAP);
        label->setContentsMargins(m);
    }
}

bool DoseStatus::iconsOnly()
{
    return DosePaint::active() && SettingsStorage::instance()->loadValue<bool>(KEY, true);
}

void DoseStatus::setIconsOnly(const bool on)
{
    SettingsStorage::instance()->storeValue(KEY, on);
}

void DoseStatus::decorate(QLabel *label, const QString &iconId)
{
    if (!label)
        return;
    label->setProperty(ICON, iconId);
    label->installEventFilter(Decorator::instance());
    applyMargin(label);
}

QString DoseStatus::text(const QString &word, const QString &value)
{
    return iconsOnly() ? value : (word + value);
}

void DoseStatus::relabel(QLabel *label, const QString &word, const QString &tooltip)
{
    if (!label)
        return;
    QString text = label->text();
    if (iconsOnly() && text.startsWith(word))
        text = text.mid(word.size());
    else if (!iconsOnly() && !text.startsWith(word))
        text = word + text;
    label->setText(text);
    if (!tooltip.isEmpty())
        label->setToolTip(iconsOnly() ? tooltip : QString());
    applyMargin(label);
    label->update();
}
