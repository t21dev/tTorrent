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

#ifdef Q_OS_WIN
namespace
{
    QString openCommand(const QString &exe)
    {
        return u"\"%1\" \"%2\""_s.arg(exe, u"%1"_s);
    }

    // Browsers and Windows only treat magnet: as a link type when a key named
    // after the scheme exists; without it, clicking a magnet link does nothing.
    // Write it only when no other app owns it (qBittorrent's installer writes
    // one under HKLM), so we never take over someone else's handler. When one
    // exists, Default apps still offers tTorrent through URLAssociations.
    void claimMagnetScheme(QSettings &classes, const QString &exe, const QString &open)
    {
        const QSettings merged {u"HKEY_CLASSES_ROOT\\magnet\\shell\\open\\command"_s, QSettings::NativeFormat};
        const QString owner = merged.value(u"Default"_s).toString();
        if (!owner.isEmpty() && !owner.contains(u"ttorrent.exe"_s, Qt::CaseInsensitive))
            return;

        classes.setValue(u"magnet/Default"_s, QCoreApplication::translate("DoseAssoc", "URL:Magnet link"));
        classes.setValue(u"magnet/URL Protocol"_s, QString());
        classes.setValue(u"magnet/DefaultIcon/Default"_s, u"\"%1\",0"_s.arg(exe));
        classes.setValue(u"magnet/shell/open/command/Default"_s, open);
    }
}
#endif

void DoseAssoc::registerForUser()
{
#ifdef Q_OS_WIN
    const QString exe = QDir::toNativeSeparators(QCoreApplication::applicationFilePath());
    const QString open = openCommand(exe);
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

    claimMagnetScheme(classes, exe, open);
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

void DoseAssoc::repairMagnetScheme()
{
#ifdef Q_OS_WIN
    // Earlier builds registered tTorrent.magnet but not the magnet key
    // itself. Add it when this exe is the one already registered, so a test
    // build run from elsewhere never takes over the installed one's links.
    const QString exe = QDir::toNativeSeparators(QCoreApplication::applicationFilePath());
    const QString open = openCommand(exe);
    QSettings classes {u"HKEY_CURRENT_USER\\Software\\Classes"_s, QSettings::NativeFormat};
    if (classes.value(u"tTorrent.magnet/shell/open/command/Default"_s).toString().compare(open, Qt::CaseInsensitive) != 0)
        return;
    if (classes.value(u"magnet/shell/open/command/Default"_s).toString().compare(open, Qt::CaseInsensitive) == 0)
        return;

    claimMagnetScheme(classes, exe, open);
    classes.sync();
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
