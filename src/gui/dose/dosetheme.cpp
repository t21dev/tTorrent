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

#include "dosetheme.h"

#include <cmath>

#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QRegularExpression>

#include "base/global.h"
#include "base/path.h"
#include "base/profile.h"

namespace
{
    // ---- Colour maths (ported from Kitab's kitab-theme/src/color.rs) ----

    double toLinear(const double c)
    {
        return (c <= 0.04045) ? (c / 12.92) : std::pow((c + 0.055) / 1.055, 2.4);
    }

    double fromLinear(const double c)
    {
        return (c <= 0.0031308) ? (c * 12.92) : ((1.055 * std::pow(c, 1.0 / 2.4)) - 0.055);
    }

    double luminance(const QColor &c)
    {
        return (0.2126 * toLinear(c.redF())) + (0.7152 * toLinear(c.greenF())) + (0.0722 * toLinear(c.blueF()));
    }

    double contrast(const QColor &a, const QColor &b)
    {
        const double la = luminance(a);
        const double lb = luminance(b);
        return (std::max(la, lb) + 0.05) / (std::min(la, lb) + 0.05);
    }

    struct Lch
    {
        double l, c, h;
    };

    // OKLab matrices from Björn Ottosson's reference implementation.
    Lch toOklch(const QColor &col)
    {
        const double r = toLinear(col.redF()), g = toLinear(col.greenF()), b = toLinear(col.blueF());
        const double l = std::cbrt((0.4122214708 * r) + (0.5363325363 * g) + (0.0514459929 * b));
        const double m = std::cbrt((0.2119034982 * r) + (0.6806995451 * g) + (0.1073969566 * b));
        const double s = std::cbrt((0.0883024619 * r) + (0.2817188376 * g) + (0.6299787005 * b));
        const double L = (0.2104542553 * l) + (0.7936177850 * m) - (0.0040720468 * s);
        const double A = (1.9779984951 * l) - (2.4285922050 * m) + (0.4505937099 * s);
        const double B = (0.0259040371 * l) + (0.7827717662 * m) - (0.8086757660 * s);
        return {L, std::hypot(A, B), std::atan2(B, A)};
    }

    QColor fromOklch(const double L, const double C, const double H)
    {
        const double A = C * std::cos(H), B = C * std::sin(H);
        const double l = std::pow(L + (0.3963377774 * A) + (0.2158037573 * B), 3);
        const double m = std::pow(L - (0.1055613458 * A) - (0.0638541728 * B), 3);
        const double s = std::pow(L - (0.0894841775 * A) - (1.2914855480 * B), 3);
        const double r = (4.0767416621 * l) - (3.3077115913 * m) + (0.2309699292 * s);
        const double g = (-1.2684380046 * l) + (2.6097574011 * m) - (0.3413193965 * s);
        const double b = (-0.0041960863 * l) - (0.7034186147 * m) + (1.7076147010 * s);
        return QColor::fromRgbF(static_cast<float>(std::clamp(fromLinear(r), 0.0, 1.0))
            , static_cast<float>(std::clamp(fromLinear(g), 0.0, 1.0))
            , static_cast<float>(std::clamp(fromLinear(b), 0.0, 1.0)));
    }

    // Keeps hue and chroma; bisects lightness for the passing value closest
    // to the original (darker on light themes, lighter on dark ones).
    QColor accentForContrast(const QColor &accent, const QColor &bg, const double target)
    {
        if (contrast(accent, bg) >= target)
            return accent;
        const Lch o = toOklch(accent);
        const bool darken = luminance(bg) > 0.18;
        double lo = darken ? 0.0 : o.l;
        double hi = darken ? o.l : 1.0;
        for (int i = 0; i < 32; ++i)
        {
            const double mid = (lo + hi) / 2;
            const bool passes = contrast(fromOklch(mid, o.c, o.h), bg) >= target;
            ((darken == passes) ? lo : hi) = mid;
        }
        return fromOklch(darken ? lo : hi, o.c, o.h);
    }

    QColor mix(const QColor &a, const QColor &b, const double t)
    {
        return QColor::fromRgbF(static_cast<float>((a.redF() * t) + (b.redF() * (1 - t)))
            , static_cast<float>((a.greenF() * t) + (b.greenF() * (1 - t)))
            , static_cast<float>((a.blueF() * t) + (b.blueF() * (1 - t))));
    }

    // ---- Themes, accents and densities ----

    // Light themes, then dark ones ("dark" starts the second group in pickers).
    const QStringList THEME_ORDER {u"light"_s, u"paper"_s, u"latte"_s, u"solarized"_s, u"blossom"_s
        , u"dark"_s, u"ink"_s, u"charcoal"_s, u"nord"_s, u"dracula"_s, u"mocha"_s, u"tokyo"_s
        , u"rosepine"_s, u"gruvbox"_s, u"vampire"_s, u"aurora"_s, u"ember"_s};

    QString themePath(const QString &id)
    {
        if (id.startsWith(u"user-"))
            return QString(DoseThemeSource::userThemeDir() + u'/' + id + u".json");
        return QString(u":/dosetheme/themes/" + id + u".json");
    }

    QJsonObject loadTheme(const QString &id)
    {
        QFile file {themePath(id)};
        if (!file.open(QIODevice::ReadOnly))
            return {};
        return QJsonDocument::fromJson(file.readAll()).object();
    }

    bool isKnown(const QString &id)
    {
        return THEME_ORDER.contains(id) || (id.startsWith(u"user-") && QFile::exists(themePath(id)));
    }

    struct Preset
    {
        QString id;
        const char *name;
        QString hex;  // empty: from the theme
    };

    const QList<Preset> &presets()
    {
        static const QList<Preset> list {
            {u"theme"_s, QT_TRANSLATE_NOOP("DoseTheme", "Theme"), {}},
            {u"amber"_s, QT_TRANSLATE_NOOP("DoseTheme", "Amber"), u"#E6B450"_s},
            {u"rose"_s, QT_TRANSLATE_NOOP("DoseTheme", "Rose"), u"#E0466A"_s},
            {u"blue"_s, QT_TRANSLATE_NOOP("DoseTheme", "Blue"), u"#5B8CFF"_s},
            {u"emerald"_s, QT_TRANSLATE_NOOP("DoseTheme", "Emerald"), u"#10B981"_s},
            {u"grape"_s, QT_TRANSLATE_NOOP("DoseTheme", "Grape"), u"#8B5CF6"_s},
            {u"wine"_s, QT_TRANSLATE_NOOP("DoseTheme", "Wine"), u"#9F234A"_s},
            {u"mono"_s, QT_TRANSLATE_NOOP("DoseTheme", "Mono"), {}},
        };
        return list;
    }

    struct Density
    {
        QString id;
        const char *name;
        int font, padV, padH, rowPad, radius, toolPad;
    };

    const QList<Density> &densityTable()
    {
        // From Kitab's DESIGN.md › Density: control height 26 / 30 / 36,
        // UI text 12.5 (rounded to 12) / 13 / 14 px, radius 7 / 8 / 9.
        static const QList<Density> list {
            {u"dense"_s, QT_TRANSLATE_NOOP("DoseTheme", "Dense"), 12, 4, 10, 4, 9, 2},
            {u"compact"_s, QT_TRANSLATE_NOOP("DoseTheme", "Compact"), 13, 6, 12, 6, 10, 3},
            {u"standard"_s, QT_TRANSLATE_NOOP("DoseTheme", "Standard"), 14, 9, 14, 9, 11, 5},
        };
        return list;
    }

    const Density &densityFor(const QString &id)
    {
        for (const Density &d : densityTable())
        {
            if (d.id == id)
                return d;
        }
        return densityTable()[1];
    }

    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseTheme", text);
    }

    QColor baseAccent(const QString &accent, const QJsonObject &colors)
    {
        if (accent.startsWith(u'#'))
        {
            const QColor c {accent};
            if (c.isValid())
                return c;
        }
        if (accent == u"mono")
            return QColor(colors.value(u"text"_s).toString());
        for (const Preset &p : presets())
        {
            if ((p.id == accent) && !p.hex.isEmpty())
                return QColor(p.hex);
        }
        return QColor(colors.value(u"accent"_s).toString());
    }
}

DoseThemeSource::DoseThemeSource(const QString &themeId, const QString &accent, const QString &density)
    : DoseThemeSource(loadTheme(isKnown(themeId) ? themeId : u"light"_s), accent, density)
{
}

DoseThemeSource::DoseThemeSource(const QJsonObject &theme, const QString &accent, const QString &density)
    : m_themeId {theme.value(u"id"_s).toString(u"draft"_s)}
    , m_density {densityFor(density).id}
    , m_icons {Path(u":/dosetheme"_s)}
{
    const QJsonObject colors = theme.value(u"colors"_s).toObject();
    m_dark = (theme.value(u"appearance"_s).toString() == u"dark");
    for (auto it = colors.begin(); it != colors.end(); ++it)
        m_tokens.insert(it.key(), QColor(it.value().toString()));

    const QColor bg = token(u"bg"_s);
    const QColor text = token(u"text"_s);
    const QColor accentColor = accentForContrast(baseAccent(accent, colors), bg, 3.0);
    m_tokens[u"accent"_s] = accentColor;
    // Text drawn on the accent (default button): whichever of bg/text reads better.
    m_tokens[u"onAccent"_s] = (contrast(bg, accentColor) >= contrast(text, accentColor)) ? bg : text;
    // Selection fill: the accent washed into the background, still readable under text.
    m_tokens[u"select"_s] = mix(accentColor, bg, m_dark ? 0.24 : 0.16);
    m_tokens[u"accentLine"_s] = mix(accentColor, bg, 0.5);
    m_tokens[u"track"_s] = mix(text, bg, m_dark ? 0.14 : 0.09);
    m_tokens[u"rowRule"_s] = mix(token(u"line"_s), bg, 0.6);
    m_tokens[u"accentHover"_s] = mix(m_dark ? QColor(Qt::white) : QColor(Qt::black), accentColor, 0.08);

    // Depth (DESIGN.md › Depth): raised controls are lit from above with a
    // darker bottom edge; fields are recessed wells; group cards sit a step up.
    const QColor white {Qt::white}, black {Qt::black};
    const QColor raisedColour = token(u"raised"_s), lineColour = token(u"line"_s);
    m_tokens[u"raisedTop"_s] = m_dark ? mix(white, raisedColour, 0.05) : raisedColour;
    m_tokens[u"raisedBottom"_s] = m_dark ? mix(black, raisedColour, 0.06) : mix(black, raisedColour, 0.035);
    m_tokens[u"edgeHi"_s] = m_dark ? mix(white, lineColour, 0.09) : lineColour;
    m_tokens[u"edgeLo"_s] = m_dark ? mix(black, lineColour, 0.45) : mix(black, lineColour, 0.16);
    m_tokens[u"well"_s] = m_dark ? mix(black, bg, 0.22) : mix(black, bg, 0.022);
    m_tokens[u"wellEdge"_s] = m_dark ? mix(black, lineColour, 0.5) : mix(black, lineColour, 0.14);
    m_tokens[u"card"_s] = mix(raisedColour, bg, m_dark ? 0.45 : 0.6);
    m_tokens[u"accentTop"_s] = mix(white, accentColor, 0.12);
    m_tokens[u"accentBottom"_s] = mix(black, accentColor, 0.10);

    const QColor muted = token(u"muted"_s), faint = token(u"faint"_s), ring = token(u"ring"_s);
    const QColor ok = token(u"ok"_s), warn = token(u"warn"_s), bad = token(u"bad"_s);
    const QColor line = token(u"line"_s), hover = token(u"hover"_s), raised = token(u"raised"_s);
    const QColor panel = token(u"panel"_s);

    m_colors = {
        {u"Palette.Window"_s, bg}, {u"Palette.WindowText"_s, text},
        {u"Palette.Base"_s, bg}, {u"Palette.AlternateBase"_s, hover},
        {u"Palette.Text"_s, text}, {u"Palette.ToolTipBase"_s, raised}, {u"Palette.ToolTipText"_s, text},
        {u"Palette.BrightText"_s, text}, {u"Palette.Highlight"_s, m_tokens[u"select"_s]},
        {u"Palette.HighlightedText"_s, text}, {u"Palette.Button"_s, raised}, {u"Palette.ButtonText"_s, text},
        {u"Palette.Link"_s, accentColor}, {u"Palette.LinkVisited"_s, accentColor},
        {u"Palette.Light"_s, raised}, {u"Palette.Midlight"_s, panel}, {u"Palette.Mid"_s, line},
        {u"Palette.Dark"_s, muted}, {u"Palette.Shadow"_s, ring},
        {u"Palette.WindowTextDisabled"_s, ring}, {u"Palette.TextDisabled"_s, ring},
        {u"Palette.ToolTipTextDisabled"_s, ring}, {u"Palette.BrightTextDisabled"_s, ring},
        {u"Palette.HighlightedTextDisabled"_s, ring}, {u"Palette.ButtonTextDisabled"_s, ring},

        {u"TransferList.Downloading"_s, accentColor}, {u"TransferList.ForcedDownloading"_s, accentColor},
        {u"TransferList.DownloadingMetadata"_s, accentColor}, {u"TransferList.ForcedDownloadingMetadata"_s, accentColor},
        {u"TransferList.Moving"_s, accentColor}, {u"TransferList.StalledDownloading"_s, muted},
        {u"TransferList.Uploading"_s, ok}, {u"TransferList.ForcedUploading"_s, ok},
        {u"TransferList.StalledUploading"_s, muted}, {u"TransferList.QueuedDownloading"_s, faint},
        {u"TransferList.QueuedUploading"_s, faint}, {u"TransferList.CheckingDownloading"_s, warn},
        {u"TransferList.CheckingUploading"_s, warn}, {u"TransferList.CheckingResumeData"_s, warn},
        {u"TransferList.StoppedDownloading"_s, faint}, {u"TransferList.StoppedUploading"_s, faint},
        {u"TransferList.MissingFiles"_s, bad}, {u"TransferList.Error"_s, bad},

        {u"PiecesBar.Border"_s, line}, {u"PiecesBar.Piece"_s, accentColor},
        {u"PiecesBar.PartialPiece"_s, m_tokens[u"accentLine"_s]}, {u"PiecesBar.MissingPiece"_s, hover},
        {u"ProgressBar"_s, accentColor},
        {u"Log.TimeStamp"_s, faint}, {u"Log.Info"_s, text}, {u"Log.Warning"_s, warn},
        {u"Log.Critical"_s, bad}, {u"Log.BannedPeer"_s, bad},
    };
}

QColor DoseThemeSource::getColor(const QString &colorId, [[maybe_unused]] const ColorMode colorMode) const
{
    // The theme decides light or dark, so both modes get the same colours.
    return m_colors.value(colorId);
}

Path DoseThemeSource::getIconPath(const QString &iconId, [[maybe_unused]] const ColorMode colorMode) const
{
    const Path base = m_icons.getIconPath(iconId, ColorMode::Light);
    // Only our own line icons are recoloured; the app icon and tray icons stay as drawn.
    if (!base.data().startsWith(u":/dosetheme/") || !base.hasExtension(u".svg"_s) || iconId.startsWith(u"qbittorrent"))
        return base;

    const QString dir = Profile::instance()->location(SpecialFolder::Cache).data()
        + u"/dose-icons/" + m_themeId + u'-' + token(u"accent"_s).name().mid(1);
    const QString out = dir + u'/' + iconId + u".svg";
    if (QFile::exists(out))
        return Path(out);

    QFile in {base.data()};
    if (!in.open(QIODevice::ReadOnly))
        return base;
    QString svg = QString::fromUtf8(in.readAll());
    // Each colour the icons are drawn in stands for a role (DESIGN.md › Icons).
    const QList<std::pair<QString, QString>> roles {
        {u"#2C2E31"_s, u"muted"_s}, {u"#1E1F22"_s, u"muted"_s}, {u"#007948"_s, u"accent"_s},
        {u"#287C42"_s, u"ok"_s}, {u"#AE6700"_s, u"warn"_s}, {u"#C53637"_s, u"bad"_s},
    };
    for (const auto &[drawn, role] : roles)
        svg.replace(drawn, token(role).name(), Qt::CaseInsensitive);
    svg.replace(u"stroke-width=\"2\""_s, u"stroke-width=\"1.6\""_s);
    svg.replace(u"stroke-width=\"2.6\""_s, u"stroke-width=\"2.1\""_s);

    QDir().mkpath(dir);
    QFile file {out};
    if (!file.open(QIODevice::WriteOnly) || (file.write(svg.toUtf8()) < 0))
        return base;
    return Path(out);
}

void DoseThemeSource::clearIconCache()
{
    QDir(Profile::instance()->location(SpecialFolder::Cache).data() + u"/dose-icons"_s).removeRecursively();
}

QByteArray DoseThemeSource::readStyleSheet()
{
    QFile file {u":/dosetheme/stylesheet.qss"_s};
    if (!file.open(QIODevice::ReadOnly))
        return {};
    QString qss = QString::fromUtf8(file.readAll());
    for (auto it = m_tokens.cbegin(); it != m_tokens.cend(); ++it)
        qss.replace(u'@' + it.key() + u'@', it.value().name());
    const Density &d = densityFor(m_density);
    qss.replace(u"@fontSmall@"_s, QString::number(d.font - 1));
    qss.replace(u"@font@"_s, QString::number(d.font));
    qss.replace(u"@padV@"_s, QString::number(d.padV));
    qss.replace(u"@padH@"_s, QString::number(d.padH));
    qss.replace(u"@rowPad@"_s, QString::number(d.rowPad));
    qss.replace(u"@radius@"_s, QString::number(d.radius));
    qss.replace(u"@toolPad@"_s, QString::number(d.toolPad));

    // Small glyphs the stylesheet points at (arrows, check marks), drawn in
    // this theme's colours into the icon cache.
    const QString dir = Profile::instance()->location(SpecialFolder::Cache).data()
        + u"/dose-icons/" + m_themeId + u'-' + token(u"accent"_s).name().mid(1);
    QDir().mkpath(dir);
    const auto asset = [&dir](const QString &name, const QString &svg) -> QString
    {
        const QString path = dir + u'/' + name + u".svg";
        QFile file {path};
        if (file.open(QIODevice::WriteOnly))
            file.write(svg.toUtf8());
        return QString(path).replace(u'\\', u'/');
    };
    const QString stroke = u"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%1' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'>%2</svg>"_s;
    const QString muted = token(u"muted"_s).name();
    const QString onAccent = token(u"onAccent"_s).name();
    qss.replace(u"@chevronDown@"_s, asset(u"qss-chevron-down"_s, stroke.arg(muted, u"<path d='m6 9 6 6 6-6'/>"_s)));
    qss.replace(u"@chevronRight@"_s, asset(u"qss-chevron-right"_s, stroke.arg(muted, u"<path d='m9 18 6-6-6-6'/>"_s)));
    qss.replace(u"@chevronUp@"_s, asset(u"qss-chevron-up"_s, stroke.arg(muted, u"<path d='m18 15-6-6-6 6'/>"_s)));
    qss.replace(u"@check@"_s, asset(u"qss-check"_s, stroke.arg(onAccent, u"<path d='M20 6 9 17l-5-5'/>"_s).replace(u"stroke-width='2'"_s, u"stroke-width='3'"_s)));
    qss.replace(u"@partial@"_s, asset(u"qss-partial"_s, stroke.arg(onAccent, u"<path d='M6 12h12'/>"_s).replace(u"stroke-width='2'"_s, u"stroke-width='3'"_s)));
    qss.replace(u"@radioDot@"_s, asset(u"qss-radio"_s, u"<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24'><circle cx='12' cy='12' r='5' fill='%1'/></svg>"_s.arg(onAccent)));
    return qss.toUtf8();
}

bool DoseThemeSource::isDark() const
{
    return m_dark;
}

QColor DoseThemeSource::token(const QString &name) const
{
    return m_tokens.value(name);
}

int DoseThemeSource::fontPixelSize() const
{
    return densityFor(m_density).font;
}

QList<DoseThemeSource::Choice> DoseThemeSource::themes()
{
    QList<Choice> list {{u"system"_s, tr(QT_TRANSLATE_NOOP("DoseTheme", "Follow system"))}};
    for (const QString &id : THEME_ORDER)
        list.append({id, loadTheme(id).value(u"name"_s).toString(id)});
    const QStringList mine = QDir(userThemeDir()).entryList({u"user-*.json"_s}, QDir::Files, QDir::Name);
    for (const QString &file : mine)
    {
        const QString id = file.chopped(5);
        list.append({id, loadTheme(id).value(u"name"_s).toString(id)});
    }
    return list;
}

QList<DoseThemeSource::Choice> DoseThemeSource::accentPresets()
{
    QList<Choice> list;
    for (const Preset &p : presets())
        list.append({p.id, tr(p.name)});
    return list;
}

QList<DoseThemeSource::Choice> DoseThemeSource::densities()
{
    QList<Choice> list;
    for (const Density &d : densityTable())
        list.append({d.id, tr(d.name)});
    return list;
}

QColor DoseThemeSource::accentBase(const QString &accent, const QString &themeId)
{
    return baseAccent(accent, loadTheme(themeId).value(u"colors"_s).toObject());
}

double DoseThemeSource::accentContrast(const QString &accent, const QString &themeId)
{
    const QJsonObject colors = loadTheme(themeId).value(u"colors"_s).toObject();
    return contrast(baseAccent(accent, colors), QColor(colors.value(u"bg"_s).toString()));
}

QString DoseThemeSource::resolveTheme(const QString &themeId, const bool systemIsDark)
{
    if (isKnown(themeId))
        return themeId;
    return systemIsDark ? u"dark"_s : u"light"_s;
}

QString DoseThemeSource::migrateAccent(const QString &accent)
{
    if (accent == u"green")
        return u"emerald"_s;
    if (accent == u"violet")
        return u"grape"_s;
    if (accent == u"ink")
        return u"mono"_s;
    return accent;
}

QString DoseThemeSource::themeKey()
{
    return u"Appearance/Theme"_s;
}

QString DoseThemeSource::accentKey()
{
    return u"Appearance/Accent"_s;
}

QString DoseThemeSource::densityKey()
{
    return u"Appearance/Density"_s;
}

QString DoseThemeSource::scaleKey()
{
    return u"Appearance/InterfaceScale"_s;
}

QString DoseThemeSource::userThemeDir()
{
    return Profile::instance()->location(SpecialFolder::Config).data() + u"/themes"_s;
}

QJsonObject DoseThemeSource::themeJson(const QString &themeId)
{
    return loadTheme(themeId);
}

QStringList DoseThemeSource::baseTokens()
{
    return {u"bg"_s, u"panel"_s, u"raised"_s, u"hover"_s, u"line"_s, u"text"_s, u"muted"_s, u"faint"_s, u"ring"_s
        , u"accent"_s, u"ok"_s, u"warn"_s, u"bad"_s};
}

double DoseThemeSource::contrastRatio(const QColor &a, const QColor &b)
{
    return contrast(a, b);
}

QString DoseThemeSource::saveUserTheme(const QString &name, const bool dark, const QHash<QString, QColor> &colours)
{
    QString slug = name.trimmed().toLower();
    slug.replace(QRegularExpression(u"[^a-z0-9]+"_s), u"-"_s);
    slug.remove(QRegularExpression(u"^-+|-+$"_s));
    if (slug.isEmpty())
        slug = u"theme"_s;
    const QString id = u"user-"_s + slug;

    QJsonObject tokens;
    for (const QString &key : baseTokens())
        tokens.insert(key, colours.value(key).name().toUpper());
    const QJsonObject theme {
        {u"id"_s, id}, {u"name"_s, name.trimmed().isEmpty() ? QString(tr("My theme")) : name.trimmed()},
        {u"appearance"_s, dark ? u"dark"_s : u"light"_s}, {u"colors"_s, tokens},
    };
    QDir().mkpath(userThemeDir());
    QFile file {themePath(id)};
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate))
        return {};
    file.write(QJsonDocument(theme).toJson(QJsonDocument::Indented));
    return id;
}
