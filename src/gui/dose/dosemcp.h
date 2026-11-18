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

#include <QString>

class QWidget;

// tTorrent for AI agents: a Model Context Protocol server inside the app
// (design: the prototype's docs/mcp.md). Streamable HTTP on
// http://127.0.0.1:7812/mcp, localhost only, off by default, one key with a
// scope: "read" (look only) or "manage" (also add, stop, start, recheck,
// categorise and remove without deleting files). Answers both the
// 2026-07-28 revision (per-request _meta) and clients that open with
// `initialize` (2025-03-26 to 2025-11-25).
namespace DoseMcp
{
    inline constexpr quint16 PORT = 7812;

    // Starts or stops the server to match the saved settings.
    void apply();

    // Settings › Web UI › "AI agents" group, and saving it.
    QWidget *settingsGroup();
    void saveSettings(QWidget *group);
}
