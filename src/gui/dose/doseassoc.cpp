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

#include "doseassoc.h"

#include <QCoreApplication>
#include <QDesktopServices>
#include <QDir>
#include <QSettings>
#include <QUrl>

#ifdef Q_OS_WIN
#include <windows.h>
#include <shlobj.h>
#endif

#include "base/global.h"

void DoseAssoc::registerForUser()
{
#ifdef Q_OS_WIN
    const QString exe = QDir::toNativeSeparators(QCoreApplication::applicationFilePath());
    const QString open = u"\"%1\" \"%2\""_s.arg(exe, u"%1"_s);
    QSettings classes {u"HKEY_CURRENT_USER\\Software\\Classes"_s, QSettings::NativeFormat};

    // The file type. Icon 1 in the exe is the .torrent file icon (qbittorrent.rc).
    classes.setValue(u"tTorrent.torrent/Default"_s, QCoreApplication::translate("DoseAssoc", "Torrent file"));
    classes.setValue(u"tTorrent.torrent/DefaultIcon/Default"_s, u"\"%1\",1"_s.arg(exe));
    classes.setValue(u"tTorrent.torrent/shell/open/command/Default"_s, open);
    classes.setValue(u".torrent/OpenWithProgids/tTorrent.torrent"_s, QString());

    // The magnet: link handler.
    classes.setValue(u"tTorrent.magnet/Default"_s, QCoreApplication::translate("DoseAssoc", "Magnet link"));
    classes.setValue(u"tTorrent.magnet/URL Protocol"_s, QString());
    classes.setValue(u"tTorrent.magnet/DefaultIcon/Default"_s, u"\"%1\",0"_s.arg(exe));
    classes.setValue(u"tTorrent.magnet/shell/open/command/Default"_s, open);
    classes.sync();

    // What Default apps lists for tTorrent.
    QSettings software {u"HKEY_CURRENT_USER\\Software"_s, QSettings::NativeFormat};
    software.setValue(u"tTorrent/Capabilities/ApplicationName"_s, u"tTorrent"_s);
    software.setValue(u"tTorrent/Capabilities/ApplicationDescription"_s
        , QCoreApplication::translate("DoseAssoc", "A fast, modern torrent client"));
    software.setValue(u"tTorrent/Capabilities/ApplicationIcon"_s, u"\"%1\",0"_s.arg(exe));
    software.setValue(u"tTorrent/Capabilities/FileAssociations/.torrent"_s, u"tTorrent.torrent"_s);
    software.setValue(u"tTorrent/Capabilities/URLAssociations/magnet"_s, u"tTorrent.magnet"_s);
    software.setValue(u"RegisteredApplications/tTorrent"_s, u"Software\\tTorrent\\Capabilities"_s);
    software.sync();

    ::SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, nullptr, nullptr);
#endif
}

void DoseAssoc::registerAndOpenSettings()
{
    registerForUser();
#ifdef Q_OS_WIN
    // Opens tTorrent's own page in Default apps (Windows 11), or the list on Windows 10.
    QDesktopServices::openUrl(QUrl(u"ms-settings:defaultapps?registeredAppUser=tTorrent"_s));
#endif
}
