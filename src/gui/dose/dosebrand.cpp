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

#include "dosebrand.h"

#include <cstring>

#include <QCoreApplication>
#include <QTranslator>

#include "base/global.h"

namespace
{
    class BrandTranslator final : public QTranslator
    {
    public:
        explicit BrandTranslator(QTranslator *inner)
            : QTranslator(QCoreApplication::instance())
            , m_inner {inner}
        {
        }

        QString translate(const char *context, const char *sourceText, const char *disambiguation, const int n) const override
        {
            if (!sourceText || !std::strstr(sourceText, "qBittorrent"))
                return {};
            QString text = m_inner ? m_inner->translate(context, sourceText, disambiguation, n) : QString();
            if (text.isEmpty())
                text = QString::fromUtf8(sourceText);
            return text.replace(u"qBittorrent"_s, u"tTorrent"_s);
        }

        bool isEmpty() const override
        {
            return false;
        }

    private:
        QTranslator *m_inner;
    };
}

void DoseBrand::install(QTranslator *appTranslator)
{
    static BrandTranslator *brand = nullptr;
    if (brand)
        return;
    brand = new BrandTranslator(appTranslator);
    QCoreApplication::installTranslator(brand);
}
