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

#include "doseabout.h"

#include <QApplication>
#include <QBoxLayout>
#include <QClipboard>
#include <QDesktopServices>
#include <QDialog>
#include <QGridLayout>
#include <QLabel>
#include <QPainter>
#include <QPushButton>
#include <QSvgRenderer>
#include <QSysInfo>
#include <QTabWidget>
#include <QTextBrowser>
#include <QUrl>

#include "base/global.h"
#include "base/path.h"
#include "base/utils/io.h"
#include "base/utils/misc.h"
#include "gui/uithememanager.h"
#include "dosetrim.h"

namespace
{
    const char STYLE[] = R"(
QWidget#doseAbout QLabel#doseAboutName { font-size: 24px; font-weight: 600; }
QWidget#doseAbout QLabel#doseAboutMeta { color: palette(dark); }
QWidget#doseAbout QTabWidget::pane { border: none; border-top: 1px solid palette(mid); }
QWidget#doseAbout QTextBrowser { border: none; background: transparent; }
QWidget#doseAbout QLabel[role="key"] { color: palette(dark); }
)";

    // The app icon: the sprout on the green tile (ttorrent/brand/icon.svg), in every theme.
    QPixmap logo(const int size, const QColor &, const QColor &, const qreal dpr)
    {
        const QString svg = QStringLiteral(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 256 256'>"
            "<rect width='256' height='256' rx='58' fill='#10a874'/>"
            "<g fill='none' stroke='#fff' stroke-width='26' stroke-linecap='round' stroke-linejoin='round'>"
            "<path d='M128 112v94'/><path d='M84 164l44 44 44-44'/></g>"
            "<path d='M128 116c0-36 22-60 70-60 0 40-22 64-70 60Z M128 116c0-26-18-46-54-46 0 30 18 50 54 46Z' fill='#fff'/></svg>");
        QSvgRenderer renderer {svg.toUtf8()};
        QPixmap pixmap {QSize(size, size) * dpr};
        pixmap.setDevicePixelRatio(dpr);
        pixmap.fill(Qt::transparent);
        QPainter painter {&pixmap};
        renderer.render(&painter, QRectF(0, 0, size, size));
        return pixmap;
    }

    QString versionLine()
    {
        return QDialog::tr("Version %1 · %2").arg(QStringLiteral(TTORRENT_VERSION), QDialog::tr(TTORRENT_VERSION_LABEL));
    }

    QString buildInfo()
    {
        return u"tTorrent %1 (%2)\nQt %3\nlibtorrent %4\nBoost %5\nOpenSSL %6\nzlib %7\n%8"_s
            .arg(QStringLiteral(TTORRENT_VERSION), QStringLiteral(TTORRENT_VERSION_LABEL), QStringLiteral(QT_VERSION_STR)
                , Utils::Misc::libtorrentVersionString(), Utils::Misc::boostVersionString()
                , Utils::Misc::opensslVersionString(), Utils::Misc::zlibVersionString(), QSysInfo::prettyProductName());
    }

    QWidget *rows(const QList<std::pair<QString, QString>> &items)
    {
        auto *page = new QWidget;
        auto *grid = new QGridLayout(page);
        grid->setContentsMargins(4, 14, 4, 4);
        grid->setHorizontalSpacing(20);
        grid->setVerticalSpacing(8);
        int row = 0;
        for (const auto &[key, value] : items)
        {
            auto *k = new QLabel(key);
            k->setProperty("role", u"key"_s);
            auto *v = new QLabel(value);
            v->setWordWrap(true);
            v->setTextInteractionFlags(Qt::TextBrowserInteraction);
            v->setOpenExternalLinks(true);
            grid->addWidget(k, row, 0, Qt::AlignTop);
            grid->addWidget(v, row++, 1, Qt::AlignTop);
        }
        grid->setColumnStretch(1, 1);
        grid->setRowStretch(row, 1);
        return page;
    }

    QWidget *text(const QString &html)
    {
        auto *label = new QLabel(html);
        label->setWordWrap(true);
        label->setTextInteractionFlags(Qt::TextBrowserInteraction);
        label->setOpenExternalLinks(true);
        label->setAlignment(Qt::AlignTop | Qt::AlignLeft);
        label->setContentsMargins(4, 14, 4, 4);
        return label;
    }
}

void DoseAbout::build(QDialog *dialog)
{
    if (!UIThemeManager::instance()->usesDoseTheme() || !dialog->layout())
        return;

    // Hide qBittorrent's own content; ours goes on top of the same layout.
    QLayout *layout = dialog->layout();
    for (int i = 0; i < layout->count(); ++i)
    {
        if (QWidget *w = layout->itemAt(i)->widget())
            w->hide();
        else if (QLayout *inner = layout->itemAt(i)->layout())
        {
            for (int j = 0; j < inner->count(); ++j)
            {
                if (QWidget *w = inner->itemAt(j)->widget())
                    w->hide();
            }
        }
    }

    auto *root = new QWidget;
    root->setObjectName(u"doseAbout"_s);
    root->setStyleSheet(QString::fromLatin1(STYLE));
    auto *col = new QVBoxLayout(root);
    col->setContentsMargins(22, 18, 22, 16);
    col->setSpacing(14);

    // Logo, name, version.
    auto *hero = new QHBoxLayout;
    hero->setSpacing(16);
    auto *mark = new QLabel;
    const QPalette pal = dialog->palette();
    mark->setPixmap(logo(64, pal.color(QPalette::WindowText), pal.color(QPalette::Window), dialog->devicePixelRatioF()));
    auto *names = new QVBoxLayout;
    names->setSpacing(2);
    auto *name = new QLabel(u"tTorrent"_s);
    name->setObjectName(u"doseAboutName"_s);
    auto *meta = new QLabel(versionLine());
    meta->setObjectName(u"doseAboutMeta"_s);
    auto *tagline = new QLabel(QDialog::tr("A fast, modern torrent client."));
    names->addWidget(name);
    names->addWidget(meta);
    names->addWidget(tagline);
    hero->addWidget(mark, 0, Qt::AlignTop);
    hero->addLayout(names, 1);
    col->addLayout(hero);

    // Links.
    auto *links = new QHBoxLayout;
    links->setSpacing(8);
    const auto link = [dialog, links](const QString &label, const QString &url)
    {
        auto *b = new QPushButton(label);
        b->setAutoDefault(false);
        QObject::connect(b, &QPushButton::clicked, dialog, [url] { QDesktopServices::openUrl(QUrl(url)); });
        links->addWidget(b);
    };
    link(QDialog::tr("Website"), QString::fromLatin1(DoseTrim::WEBSITE));
    link(QDialog::tr("Source code"), u"https://github.com/t21dev/tTorrent"_s);
    link(QDialog::tr("Report a problem"), QString::fromLatin1(DoseTrim::ISSUES));
    auto *copy = new QPushButton(QDialog::tr("Copy version info"));
    copy->setAutoDefault(false);
    QObject::connect(copy, &QPushButton::clicked, dialog, [copy]
    {
        QApplication::clipboard()->setText(buildInfo());
        copy->setText(QDialog::tr("Copied"));
    });
    links->addWidget(copy);
    links->addStretch();
    col->addLayout(links);

    // Tabs.
    auto *tabs = new QTabWidget;
    tabs->setDocumentMode(true);
    tabs->addTab(rows({
        {QDialog::tr("Website"), u"<a href=\"%1\">ttorrent.t21.dev</a>"_s.arg(QString::fromLatin1(DoseTrim::WEBSITE))},
        {QDialog::tr("Source"), u"<a href=\"https://github.com/t21dev/tTorrent\">github.com/t21dev/tTorrent</a>"_s},
        {QDialog::tr("What it is"), QDialog::tr("A torrent client that stays light over long sessions, with themes, a customizable toolbar and status bar, and the settings power users expect.")},
    }), QDialog::tr("About"));
    tabs->addTab(text(QDialog::tr(
        "<p>tTorrent began as a fork of <a href=\"https://www.qbittorrent.org\">qBittorrent</a>, and is being rebuilt from there to be leaner, faster and cleaner. "
        "Thanks to the qBittorrent project and its contributors, whose work it started from, and to its translation teams.</p>"
        "<p><b>libtorrent</b> (BitTorrent engine, BSD) · <b>Qt</b> (interface, LGPL) · <b>QWindowKit</b> (title bar, Apache-2.0) · "
        "<b>Boost</b>, <b>OpenSSL</b>, <b>zlib</b> · <b>Instrument Sans</b> and <b>DM Mono</b> (fonts, OFL) · <b>Lucide</b> (icon shapes, ISC) · "
        "theme colours from <b>Nord</b>, <b>Dracula</b>, <b>Catppuccin</b>, <b>Tokyo Night</b>, <b>Rosé Pine</b>, <b>Gruvbox</b> and <b>Solarized</b> (MIT).</p>"
        "<p>Peer countries use the free IP to Country Lite database by <a href=\"https://db-ip.com/\">DB-IP</a>, "
        "licensed under Creative Commons Attribution 4.0.</p>")),
        QDialog::tr("Credits"));
    auto *licence = new QTextBrowser;
    licence->setOpenExternalLinks(true);
    if (const auto read = Utils::IO::readFile(Path(u":/gpl.html"_s), -1, QIODevice::Text))
    {
        licence->setHtml(QDialog::tr("<p>tTorrent is free software under the GNU General Public License: source files version 2 or later, "
            "the program version 3 or later.</p>") + QString::fromUtf8(read.value()));
    }
    tabs->addTab(licence, QDialog::tr("Licence"));
    tabs->addTab(rows({
        {u"tTorrent"_s, u"%1 (%2)"_s.arg(QStringLiteral(TTORRENT_VERSION), QDialog::tr(TTORRENT_VERSION_LABEL))},
        {u"Qt"_s, QStringLiteral(QT_VERSION_STR)},
        {u"libtorrent"_s, Utils::Misc::libtorrentVersionString()},
        {u"Boost"_s, Utils::Misc::boostVersionString()},
        {u"OpenSSL"_s, Utils::Misc::opensslVersionString()},
        {u"zlib"_s, Utils::Misc::zlibVersionString()},
        {QDialog::tr("System"), QSysInfo::prettyProductName()},
    }), QDialog::tr("Build info"));
    col->addWidget(tabs, 1);

    // Footer.
    auto *footer = new QHBoxLayout;
    auto *copyright = new QLabel(QDialog::tr("© 2026 t21 dev. GPL."));
    copyright->setObjectName(u"doseAboutMeta"_s);
    auto *close = new QPushButton(QDialog::tr("Close"));
    close->setDefault(true);
    QObject::connect(close, &QPushButton::clicked, dialog, &QDialog::accept);
    footer->addWidget(copyright);
    footer->addStretch();
    footer->addWidget(close);
    col->addLayout(footer);

    if (auto *box = qobject_cast<QBoxLayout *>(layout))
        box->insertWidget(0, root, 1);
    else
        layout->addWidget(root);
    dialog->setWindowTitle(QDialog::tr("About tTorrent"));
    dialog->resize(640, 560);
}
