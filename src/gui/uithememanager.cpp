/*
 * Bittorrent Client using Qt and libtorrent.
 * Copyright (C) 2023-2026  Vladimir Golovnev <glassez@yandex.ru>
 * Copyright (C) 2019, 2021  Prince Gupta <jagannatharjun11@gmail.com>
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
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301,
 * USA.
 *
 * In addition, as a special exception, the copyright holders give permission to
 * link this program with the OpenSSL project's "OpenSSL" library (or with
 * modified versions of it that use the same license as the "OpenSSL" library),
 * and distribute the linked executables. You must obey the GNU General Public
 * License in all respects for all of the code used other than "OpenSSL".  If
 * you modify file(s), you may extend this exception to your version of the
 * file(s), but you are not obligated to do so. If you do not wish to do so,
 * delete this exception statement from your version.
 */

#include "uithememanager.h"

#include "base/settingsstorage.h"
#include "dose/dosestyle.h"
#include "dose/dosetheme.h"

#include <QFontDatabase>
#include <QApplication>
#include <QPalette>
#include <QPixmapCache>
#include <QResource>
#include <QStyle>
#include <QStyleHints>

#include "base/logger.h"
#include "base/path.h"
#include "base/profile.h"
#include "base/preferences.h"
#include "uithemecommon.h"

#ifdef Q_OS_WIN
#include "base/utils/reg.h"
#endif

using namespace Qt::Literals::StringLiterals;

namespace
{
    bool isDarkTheme()
    {
        const QPalette palette = qApp->palette();
        const QColor &color = palette.color(QPalette::Active, QPalette::Base);
        return (color.lightness() < 127);
    }

    Path resolveThemePath(const Path &themePath)
    {
        return (themePath.isAbsolute() ? themePath : (Profile::instance()->location(SpecialFolder::Config) / themePath));
    }
}

UIThemeManager *UIThemeManager::m_instance = nullptr;

void UIThemeManager::freeInstance()
{
    delete m_instance;
    m_instance = nullptr;
}

void UIThemeManager::initInstance()
{
    if (!m_instance)
        m_instance = new UIThemeManager;
}

UIThemeManager::UIThemeManager()
    : m_useCustomTheme {Preferences::instance()->useCustomUITheme()}
#ifdef QBT_HAS_COLORSCHEME_OPTION
    , m_colorSchemeSetting {u"Appearance/ColorScheme"_s}
#endif
    , m_trayIconStyleSetting {u"Appearance/TrayIconStyle"_s}
#if (defined(Q_OS_UNIX) && !defined(Q_OS_MACOS))
    , m_useSystemIcons {Preferences::instance()->useSystemIcons()}
#endif
{
#ifdef Q_OS_WIN
    updateSystemColorMode();
#endif

    if (const QString styleName = Preferences::instance()->getStyle(); styleName.compare(u"system", Qt::CaseInsensitive) != 0)
    {
        if (!QApplication::setStyle(styleName))
            LogMsg(tr("Set app style failed. Unknown style: \"%1\"").arg(styleName), Log::WARNING);
    }

#ifdef QBT_HAS_COLORSCHEME_OPTION
    applyColorScheme();
#endif

    connect(QApplication::styleHints(), &QStyleHints::colorSchemeChanged, this, &UIThemeManager::onColorSchemeChanged);

    if (m_useCustomTheme)
    {
        const Path themePath = resolveThemePath(Preferences::instance()->customUIThemePath());

        if (themePath.hasExtension(u".qbtheme"_s))
        {
            if (QResource::registerResource(themePath.data(), u"/uitheme"_s))
                m_themeSource = std::make_unique<QRCThemeSource>();
            else
                LogMsg(tr("Failed to load UI theme from file: \"%1\"").arg(themePath.toString()), Log::WARNING);
        }
        else if (themePath.filename() == CONFIG_FILE_NAME)
        {
            m_themeSource = std::make_unique<FolderThemeSource>(themePath.parentPath());
        }
    }

    // Dose is this build's default look (src/gui/dose/theme): used whenever no
    // custom theme is chosen or the chosen one can't be loaded.
    const bool doseDefault = !m_themeSource;
    if (doseDefault)
    {
        for (const QString &font : {u":/dosetheme/fonts/InstrumentSans-Regular.ttf"_s
                , u":/dosetheme/fonts/InstrumentSans-Medium.ttf"_s
                , u":/dosetheme/fonts/InstrumentSans-SemiBold.ttf"_s
                , u":/dosetheme/fonts/DMMono-Regular.ttf"_s, u":/dosetheme/fonts/DMMono-Medium.ttf"_s})
        {
            QFontDatabase::addApplicationFont(font);
        }
        m_doseTheme = true;
        DoseThemeSource::clearIconCache();
        applyDose();
    }
    else if (m_useCustomTheme)
    {
        applyPalette();
        applyStyleSheet();
    }
}

bool UIThemeManager::systemIsDark() const
{
#ifdef Q_OS_WIN
    return (m_systemColorMode == ColorMode::Dark);
#else
    return (QGuiApplication::styleHints()->colorScheme() == Qt::ColorScheme::Dark);
#endif
}

void UIThemeManager::applyDose()
{
    const QString theme = lookTheme();
    m_doseResolved = DoseThemeSource::resolveTheme(theme, systemIsDark());
    DoseStyle::install();
    auto source = std::make_unique<DoseThemeSource>(m_doseResolved, lookAccent(), lookDensity());
    const bool dark = source->isDark();
    m_themeSource = std::move(source);
    m_icons.clear();
    m_darkModeIcons.clear();
    QPixmapCache::clear();
    // Native parts (scroll arrows, dialogs Qt draws itself) follow the theme;
    // with "Follow system" the scheme is left to the system so we hear changes.
    if (theme == u"system")
        qApp->styleHints()->unsetColorScheme();
    else
        qApp->styleHints()->setColorScheme(dark ? Qt::ColorScheme::Dark : Qt::ColorScheme::Light);
    applyPalette();
    applyStyleSheet();
}

void UIThemeManager::setLook(const QString &theme, const QString &accent, const QString &density)
{
    SettingsStorage *settings = SettingsStorage::instance();
    settings->storeValue(DoseThemeSource::themeKey(), theme);
    settings->storeValue(DoseThemeSource::accentKey(), accent);
    settings->storeValue(DoseThemeSource::densityKey(), density);
    if (!m_doseTheme)
        return;
    applyDose();
    emit themeChanged();
}

QString UIThemeManager::lookTheme() const
{
    return SettingsStorage::instance()->loadValue<QString>(DoseThemeSource::themeKey(), u"system"_s);
}

QString UIThemeManager::lookAccent() const
{
    return DoseThemeSource::migrateAccent(
        SettingsStorage::instance()->loadValue<QString>(DoseThemeSource::accentKey(), u"theme"_s));
}

QString UIThemeManager::lookDensity() const
{
    return SettingsStorage::instance()->loadValue<QString>(DoseThemeSource::densityKey(), u"compact"_s);
}

QString UIThemeManager::resolvedTheme() const
{
    return m_doseResolved;
}

bool UIThemeManager::usesDoseTheme() const
{
    return m_doseTheme;
}

QColor UIThemeManager::doseToken(const QString &name) const
{
    const auto *dose = dynamic_cast<const DoseThemeSource *>(m_themeSource.get());
    return dose ? dose->token(name) : QColor();
}

UIThemeManager *UIThemeManager::instance()
{
    return m_instance;
}

#ifdef QBT_HAS_COLORSCHEME_OPTION
ColorScheme UIThemeManager::colorScheme() const
{
    return m_colorSchemeSetting.get(ColorScheme::System);
}

void UIThemeManager::setColorScheme(const ColorScheme value)
{
    if (value == colorScheme())
        return;

    m_colorSchemeSetting = value;
}

void UIThemeManager::applyColorScheme() const
{
    switch (colorScheme())
    {
    case ColorScheme::System:
    default:
        qApp->styleHints()->unsetColorScheme();
        break;
    case ColorScheme::Light:
        qApp->styleHints()->setColorScheme(Qt::ColorScheme::Light);
        break;
    case ColorScheme::Dark:
        qApp->styleHints()->setColorScheme(Qt::ColorScheme::Dark);
        break;
    }
}
#endif

TrayIconStyle UIThemeManager::trayIconStyle() const
{
    return m_trayIconStyleSetting.get(TrayIconStyle::Normal);
}

void UIThemeManager::setTrayIconStyle(const TrayIconStyle value)
{
    if (value == trayIconStyle())
        return;

    m_trayIconStyleSetting = value;
}

void UIThemeManager::applyStyleSheet() const
{
    qApp->setStyleSheet(QString::fromUtf8(m_themeSource->readStyleSheet()));
}

void UIThemeManager::onColorSchemeChanged()
{
    // "Follow system": the system switched, so switch theme with it.
    if (m_doseTheme && (lookTheme() == u"system")
        && (DoseThemeSource::resolveTheme(u"system"_s, systemIsDark()) != m_doseResolved))
    {
        applyDose();
    }
    emit themeChanged();

    // workaround to refresh styled controls once color scheme is changed
    // tTorrent: keep the DoseStyle layer when re-creating the style.
    if (m_doseTheme)
        QApplication::setStyle(new DoseStyle(QApplication::style()->name()));
    else
        QApplication::setStyle(QApplication::style()->name());
}

QIcon UIThemeManager::getIcon(const QString &iconId, [[maybe_unused]] const QString &fallback, const ColorMode colorMode) const
{
    auto &icons = (colorMode == ColorMode::Dark) ? m_darkModeIcons : m_icons;

    const auto iter = icons.find(iconId);
    if (iter != icons.end())
        return *iter;

#if (defined(Q_OS_UNIX) && !defined(Q_OS_MACOS))
    // Don't cache system icons because users might change them at run time
    if (m_useSystemIcons)
    {
        auto icon = QIcon::fromTheme(iconId);
        if (icon.isNull() || icon.availableSizes().isEmpty())
            icon = QIcon::fromTheme(fallback, QIcon(m_themeSource->getIconPath(iconId, colorMode).data()));
        return icon;
    }
#endif

    const QIcon icon {m_themeSource->getIconPath(iconId, colorMode).data()};
    icons[iconId] = icon;
    return icon;
}

QIcon UIThemeManager::getIcon(const QString &iconId, const QString &fallback) const
{
    const auto colorMode = isDarkTheme() ? ColorMode::Dark : ColorMode::Light;
    return getIcon(iconId, fallback, colorMode);
}

QIcon UIThemeManager::getSystrayIcon() const
{
#ifdef Q_OS_WIN
    const auto colorMode = m_systemColorMode;
#else
    const auto colorMode = (qApp->styleHints()->colorScheme() == Qt::ColorScheme::Dark)
            ? ColorMode::Dark : ColorMode::Light;
#endif
    const QString fallback = (colorMode == ColorMode::Light)
            ? u"qbittorrent-tray-light"_s : u"qbittorrent-tray-dark"_s;

    switch (trayIconStyle())
    {
    case TrayIconStyle::Normal:
        return getIcon(u"qbittorrent-tray"_s, {}, colorMode);

    case TrayIconStyle::Monochrome:
        return getIcon(u"qbittorrent-tray-mono"_s, fallback, colorMode);

    case TrayIconStyle::MonochromeFilled: // tTorrent
        return getIcon((colorMode == ColorMode::Light) ? u"qbittorrent-tray-filled-light"_s : u"qbittorrent-tray-filled-dark"_s, fallback, colorMode);
    }

    Q_UNREACHABLE_RETURN({});
}

QIcon UIThemeManager::getFlagIcon(const QString &countryIsoCode) const
{
    if (countryIsoCode.isEmpty())
        return {};

    const QString key = countryIsoCode.toLower();
    const auto iter = m_flags.constFind(key);
    if (iter != m_flags.cend())
        return *iter;

    const QIcon icon {u":/icons/flags/" + key + u".svg"};
    m_flags[key] = icon;
    return icon;
}

QPixmap UIThemeManager::getScaledPixmap(const QString &iconId, const int height) const
{
    // (workaround) svg images require the use of `QIcon()` to load and scale losslessly,
    // otherwise other image classes will convert it to pixmap first and follow-up scaling will become lossy.

    Q_ASSERT(height > 0);

    const QString cacheKey = iconId + u'@' + QString::number(height);

    QPixmap pixmap;
    if (!QPixmapCache::find(cacheKey, &pixmap))
    {
        pixmap = getIcon(iconId).pixmap(height);
        QPixmapCache::insert(cacheKey, pixmap);
    }

    return pixmap;
}

QColor UIThemeManager::getColor(const QString &id) const
{
    const QColor color = m_themeSource->getColor(id, (isDarkTheme() ? ColorMode::Dark : ColorMode::Light));
    return color;
}

#ifdef Q_OS_WIN
void UIThemeManager::updateSystemColorMode()
{
    const HKEY hkRoot = HKEY_CURRENT_USER;
    const REGSAM samDesired = KEY_READ;

    HKEY hkPersonalize = nullptr;
    const LSTATUS status = ::RegOpenKeyExW(hkRoot, L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize", 0, samDesired, &hkPersonalize);
    if (status != ERROR_SUCCESS)
        return;

    [[maybe_unused]] const auto hkPersonalizeGuard = qScopeGuard([&hkPersonalize] { ::RegCloseKey(hkPersonalize); });

    const ulong val = Utils::Reg::getULongValue(hkPersonalize, u"SystemUsesLightTheme"_s);
    const auto systemUsesLightTheme = static_cast<bool>(val);
    const auto systemColorMode = systemUsesLightTheme ? ColorMode::Light : ColorMode::Dark;
    if (systemColorMode != m_systemColorMode)
    {
        m_systemColorMode = systemColorMode;
        onColorSchemeChanged();
    }
}
#endif

void UIThemeManager::applyPalette() const
{
    struct ColorDescriptor
    {
        QString id;
        QPalette::ColorRole colorRole;
        QPalette::ColorGroup colorGroup;
    };

    const ColorDescriptor paletteColorDescriptors[] =
    {
        {u"Palette.Window"_s, QPalette::Window, QPalette::Normal},
        {u"Palette.WindowText"_s, QPalette::WindowText, QPalette::Normal},
        {u"Palette.Base"_s, QPalette::Base, QPalette::Normal},
        {u"Palette.AlternateBase"_s, QPalette::AlternateBase, QPalette::Normal},
        {u"Palette.Text"_s, QPalette::Text, QPalette::Normal},
        {u"Palette.ToolTipBase"_s, QPalette::ToolTipBase, QPalette::All},
        {u"Palette.ToolTipText"_s, QPalette::ToolTipText, QPalette::All},
        {u"Palette.BrightText"_s, QPalette::BrightText, QPalette::Normal},
        {u"Palette.Highlight"_s, QPalette::Highlight, QPalette::Normal},
        {u"Palette.HighlightedText"_s, QPalette::HighlightedText, QPalette::Normal},
        {u"Palette.Button"_s, QPalette::Button, QPalette::Normal},
        {u"Palette.ButtonText"_s, QPalette::ButtonText, QPalette::Normal},
        {u"Palette.Link"_s, QPalette::Link, QPalette::Normal},
        {u"Palette.LinkVisited"_s, QPalette::LinkVisited, QPalette::Normal},
        {u"Palette.Light"_s, QPalette::Light, QPalette::Normal},
        {u"Palette.Midlight"_s, QPalette::Midlight, QPalette::Normal},
        {u"Palette.Mid"_s, QPalette::Mid, QPalette::Normal},
        {u"Palette.Dark"_s, QPalette::Dark, QPalette::Normal},
        {u"Palette.Shadow"_s, QPalette::Shadow, QPalette::Normal},
        {u"Palette.WindowTextDisabled"_s, QPalette::WindowText, QPalette::Disabled},
        {u"Palette.TextDisabled"_s, QPalette::Text, QPalette::Disabled},
        {u"Palette.ToolTipTextDisabled"_s, QPalette::ToolTipText, QPalette::Disabled},
        {u"Palette.BrightTextDisabled"_s, QPalette::BrightText, QPalette::Disabled},
        {u"Palette.HighlightedTextDisabled"_s, QPalette::HighlightedText, QPalette::Disabled},
        {u"Palette.ButtonTextDisabled"_s, QPalette::ButtonText, QPalette::Disabled}
    };

    QPalette palette = qApp->palette();
    for (const ColorDescriptor &colorDescriptor : paletteColorDescriptors)
    {
        // For backward compatibility, the palette color overrides are read from the section of the "light mode" colors
        const QColor newColor = m_themeSource->getColor(colorDescriptor.id, ColorMode::Light);
        if (newColor.isValid())
            palette.setColor(colorDescriptor.colorGroup, colorDescriptor.colorRole, newColor);
    }

    if (m_doseTheme)
    {
        // Same colours whether or not the window has focus.
        for (int role = 0; role < QPalette::NColorRoles; ++role)
        {
            const auto r = static_cast<QPalette::ColorRole>(role);
            palette.setColor(QPalette::Inactive, r, palette.color(QPalette::Active, r));
        }
        if (const auto *dose = dynamic_cast<const DoseThemeSource *>(m_themeSource.get()))
            palette.setColor(QPalette::All, QPalette::PlaceholderText, dose->token(u"faint"_s));
    }

    qApp->setPalette(palette);
}
