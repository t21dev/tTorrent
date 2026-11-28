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
#include <QFileInfo>
#include <QRegularExpression>
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

    bool samePath(const QString &a, const QString &b)
    {
        return QDir::cleanPath(QDir::fromNativeSeparators(a)).compare(QDir::cleanPath(QDir::fromNativeSeparators(b)), Qt::CaseInsensitive) == 0;
    }

    // The copy the installer put in place: the folder its uninstall entry
    // names, or Program Files\tTorrent (install-local.ps1 writes no entry).
    bool isInstalledCopy(const QString &exeDir)
    {
        const QSettings uninstall {u"HKEY_LOCAL_MACHINE\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\tTorrent"_s
            , QSettings::Registry64Format};
        const QString location = uninstall.value(u"InstallLocation"_s).toString();
        if (!location.isEmpty() && samePath(location, exeDir))
            return true;
        const QString programFiles = qEnvironmentVariable("ProgramW6432", qEnvironmentVariable("ProgramFiles"));
        return !programFiles.isEmpty() && samePath(programFiles + u"\\tTorrent"_s, exeDir);
    }

    // Replaces a ttorrent.exe path in the value with exe when that file is
    // gone. Arguments and icon indexes around it stay as they are.
    bool repointValue(QSettings &settings, const QString &key, const QString &exe)
    {
        static const QRegularExpression pathRx {u"[A-Za-z]:\\\\[^\"]*?\\\\ttorrent\\.exe"_s, QRegularExpression::CaseInsensitiveOption};
        const QString value = settings.value(key).toString();
        const QRegularExpressionMatch match = pathRx.match(value);
        if (!match.hasMatch())
            return false;
        const QString path = match.captured();
        if (samePath(path, exe) || QFileInfo::exists(path))
            return false;
        settings.setValue(key, QString(value).replace(match.capturedStart(), match.capturedLength(), exe));
        return true;
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

void DoseAssoc::repairMovedInstall()
{
#ifdef Q_OS_WIN
    const QString exe = QDir::toNativeSeparators(QCoreApplication::applicationFilePath());
    if (!isInstalledCopy(QFileInfo(exe).absolutePath()))
        return;

    // torrent_auto_file and Applications\ttorrent.exe are what Windows writes
    // for "Open with › Choose another app"; Default apps' choice (UserChoice)
    // often names one of them, so it breaks with them.
    bool repaired = false;
    QSettings classes {u"HKEY_CURRENT_USER\\Software\\Classes"_s, QSettings::NativeFormat};
    for (const QString &progId : {u"tTorrent.torrent"_s, u"tTorrent.magnet"_s, u"magnet"_s
            , u"torrent_auto_file"_s, u"magnet_auto_file"_s, u"Applications/ttorrent.exe"_s})
    {
        repaired |= repointValue(classes, progId + u"/shell/open/command/Default"_s, exe);
        repaired |= repointValue(classes, progId + u"/DefaultIcon/Default"_s, exe);
    }
    classes.sync();

    QSettings software {u"HKEY_CURRENT_USER\\Software"_s, QSettings::NativeFormat};
    repaired |= repointValue(software, u"tTorrent/Capabilities/ApplicationIcon"_s, exe);
    repaired |= repointValue(software, u"Microsoft/Windows/CurrentVersion/Run/tTorrent"_s, exe);
    software.sync();

    if (!repaired)
        return;
    // tTorrent handled these at the old path, and the old uninstaller may have
    // removed its own keys while moving: register this exe again.
    registerForUser();
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
