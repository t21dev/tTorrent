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

#include <QMutex>
#include <QMutexLocker>
#include <QSet>
#include <QString>

namespace Utils
{
    // tTorrent: one shared copy of strings that repeat across thousands of
    // objects (tracker messages such as "timed out", local endpoint names).
    // QString is implicitly shared, so returning the pooled copy frees the
    // duplicate. Thread-safe; the pool empties itself past 8192 entries so
    // one-off messages can't grow it without bound.
    inline QString intern(const QString &text)
    {
        if (text.isEmpty())
            return {};
        static QMutex mutex;
        static QSet<QString> pool;
        const QMutexLocker locker {&mutex};
        if (const auto it = pool.constFind(text); it != pool.cend())
            return *it;
        if (pool.size() >= 8192)
            pool.clear();
        pool.insert(text);
        return text;
    }
}
