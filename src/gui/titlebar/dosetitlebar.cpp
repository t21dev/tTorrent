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

#include "dosetitlebar.h"

#include <QApplication>
#include <QDialog>
#include <QEvent>
#include <QFile>
#include <QHBoxLayout>
#include <QIcon>
#include <QLabel>
#include <QTimer>
#include <QLayout>
#include <QMainWindow>
#include <QMenuBar>
#include <QPainter>
#include <QPixmap>
#include <QSvgRenderer>

#include <QWKWidgets/widgetwindowagent.h>

#include "windowbar.h"
#include "windowbutton.h"

namespace
{
    // The title bar's look. Colours come from the palette, so it follows the
    // light and dark palettes of the Dose theme (and any other theme).
    const char STYLE[] = R"(
QWK--WindowBar {
    background: palette(window);
    border-bottom: 1px solid palette(mid);
}
QWK--WindowBar QLabel#win-title-label {
    color: palette(dark);
    padding: 0 8px;
}
QWK--WindowBar QLabel#win-title-label[doseHosting="true"] {
    color: transparent;
}
QWK--WindowBar#dose-dialog-bar QLabel#win-title-label {
    color: palette(window-text);
    font-weight: 600;
    padding: 0 14px;
}
QWK--WindowBar#dose-dialog-bar {
    border-bottom: none;
}
QWK--WindowBar QMenuBar#win-menu-bar {
    background: transparent;
    border: none;
    padding: 5px 2px 0 2px;
}
QWK--WindowBar QMenuBar#win-menu-bar::item {
    padding: 5px 10px;
    border-radius: 6px;
    background: transparent;
}
QWK--WindowBar QMenuBar#win-menu-bar::item:selected {
    background: palette(alternate-base);
}
QWK--WindowButton, QWK--WindowButton:default, QWK--WindowButton:flat {
    border: none;
    border-radius: 0;
    background: transparent;
    min-width: 46px;
    max-width: 46px;
    min-height: 38px;
    max-height: 38px;
    padding: 0;
    margin: 0;
}
QWK--WindowButton#icon-button {
    min-width: 40px;
    max-width: 40px;
}
QWK--WindowButton#icon-button:hover { background: transparent; }
QWK--WindowButton:hover {
    background: palette(alternate-base);
}
QWK--WindowButton:pressed {
    background: palette(midlight);
}
QWK--WindowButton#close-button:hover {
    background: #C42B1C;
}
)";

    // An SVG icon drawn in a colour (the files use currentColor).
    QIcon tinted(const QString &name, const QColor &colour)
    {
        QFile file {QStringLiteral(":/titlebar/icons/") + name + QStringLiteral(".svg")};
        if (!file.open(QIODevice::ReadOnly))
            return {};
        QByteArray svg = file.readAll();
        svg.replace("currentColor", colour.name().toLatin1());
        QSvgRenderer renderer {svg};
        QIcon icon;
        for (const qreal scale : {1.0, 1.25, 1.5, 2.0})
        {
            QPixmap pixmap {QSize(16, 16) * scale};
            pixmap.setDevicePixelRatio(scale);
            pixmap.fill(Qt::transparent);
            QPainter painter {&pixmap};
            renderer.render(&painter, QRectF(0, 0, 16, 16));
            icon.addPixmap(pixmap);
        }
        return icon;
    }

    // The app icon: the sprout on the green tile (ttorrent/brand/icon.svg),
    // the same in every theme so the title bar matches the taskbar.
    QIcon logo(const QColor &, const QColor &)
    {
        const QString svg = QStringLiteral(
            "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 256 256'>"
            "<rect width='256' height='256' rx='58' fill='#10a874'/>"
            "<g fill='none' stroke='#fff' stroke-width='26' stroke-linecap='round' stroke-linejoin='round'>"
            "<path d='M128 112v94'/><path d='M84 164l44 44 44-44'/></g>"
            "<path d='M128 116c0-36 22-60 70-60 0 40-22 64-70 60Z M128 116c0-26-18-46-54-46 0 30 18 50 54 46Z' fill='#fff'/></svg>");
        QSvgRenderer renderer {svg.toUtf8()};
        QIcon icon;
        for (const qreal scale : {1.0, 1.25, 1.5, 2.0})
        {
            QPixmap pixmap {QSize(18, 18) * scale};
            pixmap.setDevicePixelRatio(scale);
            pixmap.fill(Qt::transparent);
            QPainter painter {&pixmap};
            renderer.render(&painter, QRectF(0, 0, 18, 18));
            icon.addPixmap(pixmap);
        }
        return icon;
    }

    // Redraws the button icons when the palette changes (light/dark).
    class PaletteWatcher final : public QObject
    {
    public:
        PaletteWatcher(QWidget *bar, QWK::WindowButton *min, QWK::WindowButton *max,
                QWK::WindowButton *close, QWK::WindowButton *icon = nullptr)
            : QObject(bar), m_bar {bar}, m_min {min}, m_max {max}, m_close {close}, m_icon {icon}
        {
            bar->installEventFilter(this);
            apply();
        }

        void apply()
        {
            const QColor ink = m_bar->palette().color(QPalette::WindowText);
            if (m_min)
                m_min->setIconNormal(tinted(QStringLiteral("minimize"), ink));
            if (m_max)
            {
                m_max->setIconNormal(tinted(QStringLiteral("maximize"), ink));
                m_max->setIconChecked(tinted(QStringLiteral("restore"), ink));
            }
            m_close->setIconNormal(tinted(QStringLiteral("close"), ink));
            if (m_icon)
                m_icon->setIconNormal(logo(ink, m_bar->palette().color(QPalette::Window)));
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if ((watched == m_bar) && (event->type() == QEvent::PaletteChange))
                apply();
            return QObject::eventFilter(watched, event);
        }

    private:
        QWidget *m_bar;
        QWK::WindowButton *m_min;
        QWK::WindowButton *m_max;
        QWK::WindowButton *m_close;
        QWK::WindowButton *m_icon;
    };

    // Qt keeps a button hovered after a click maximizes the window under it.
    void sendLeave(QWidget *widget)
    {
        QEvent leave {QEvent::Leave};
        QApplication::sendEvent(widget, &leave);
    }
}

void DoseTitleBar::install(QMainWindow *window, QMenuBar *menuBar)
{
    auto *agent = new QWK::WidgetWindowAgent(window);
    agent->setup(window);

    auto *bar = new QWK::WindowBar;
    bar->setStyleSheet(QString::fromLatin1(STYLE));

    auto *title = new QLabel;
    title->setAlignment(Qt::AlignCenter);
    title->setObjectName(QStringLiteral("win-title-label"));
    bar->setTitleLabel(title);

#ifndef Q_OS_MACOS
    // Take the menu bar out of the window's layout without deleting it; the
    // title bar holds it from here on.
    window->layout()->setMenuBar(nullptr);
    menuBar->setObjectName(QStringLiteral("win-menu-bar"));
    // Its natural height, so the bar's layout centres it beside the logo.
    menuBar->setSizePolicy(menuBar->sizePolicy().horizontalPolicy(), QSizePolicy::Fixed);
    bar->setMenuBar(menuBar);

    auto *iconButton = new QWK::WindowButton;
    iconButton->setObjectName(QStringLiteral("icon-button"));
    iconButton->setIconSize({18, 18});
    auto *minButton = new QWK::WindowButton;
    minButton->setObjectName(QStringLiteral("min-button"));
    minButton->setProperty("system-button", true);
    auto *maxButton = new QWK::WindowButton;
    maxButton->setObjectName(QStringLiteral("max-button"));
    maxButton->setProperty("system-button", true);
    maxButton->setCheckable(true);
    auto *closeButton = new QWK::WindowButton;
    closeButton->setObjectName(QStringLiteral("close-button"));
    closeButton->setProperty("system-button", true);
    bar->setIconButton(iconButton);
    bar->setMinButton(minButton);
    bar->setMaxButton(maxButton);
    bar->setCloseButton(closeButton);
    new PaletteWatcher(bar, minButton, maxButton, closeButton, iconButton);
#else
    Q_UNUSED(menuBar);
#endif

    bar->setHostWidget(window);
    agent->setTitleBar(bar);

#ifndef Q_OS_MACOS
    agent->setSystemButton(QWK::WindowAgentBase::WindowIcon, iconButton);
    agent->setSystemButton(QWK::WindowAgentBase::Minimize, minButton);
    agent->setSystemButton(QWK::WindowAgentBase::Maximize, maxButton);
    agent->setSystemButton(QWK::WindowAgentBase::Close, closeButton);
    agent->setHitTestVisible(menuBar, true);

    QObject::connect(bar, &QWK::WindowBar::minimizeRequested, window, &QWidget::showMinimized);
    QObject::connect(bar, &QWK::WindowBar::maximizeRequested, window, [window, maxButton](const bool max)
    {
        if (max)
            window->showMaximized();
        else
            window->showNormal();
        sendLeave(maxButton);
    });
    QObject::connect(bar, &QWK::WindowBar::closeRequested, window, &QWidget::close);
#else
    // Leave room for the traffic lights on the left.
    agent->setSystemButtonAreaCallback([](const QSize &size)
    {
        return QRect(QPoint(0, 0), QSize(75, size.height()));
    });
#endif

    window->setMenuWidget(bar);
}

void DoseTitleBar::installDialog(QDialog *dialog)
{
#ifdef Q_OS_MACOS
    Q_UNUSED(dialog);
#else
    QLayout *layout = dialog->layout();
    if (!layout || layout->menuBar())
        return;

    auto *agent = new QWK::WidgetWindowAgent(dialog);
    agent->setup(dialog);

    auto *bar = new QWK::WindowBar;
    bar->setObjectName(QStringLiteral("dose-dialog-bar"));
    bar->setStyleSheet(QString::fromLatin1(STYLE));
    auto *title = new QLabel;
    title->setObjectName(QStringLiteral("win-title-label"));
    title->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
    bar->setTitleLabel(title);
    bar->setTitleFollowWindow(false);
    // The dialog's title on the left, unless the dialog shows its own
    // heading (property doseOwnHeading), so the name never appears twice.
    const auto showTitle = [dialog, title]
    {
        title->setText(dialog->property("doseOwnHeading").toBool() ? QString() : dialog->windowTitle());
    };
    showTitle();
    QObject::connect(dialog, &QWidget::windowTitleChanged, title, showTitle);
    QTimer::singleShot(0, title, showTitle);

    auto *closeButton = new QWK::WindowButton;
    closeButton->setObjectName(QStringLiteral("close-button"));
    closeButton->setProperty("system-button", true);
    bar->setCloseButton(closeButton);
    new PaletteWatcher(bar, nullptr, nullptr, closeButton);

    bar->setHostWidget(dialog);
    agent->setTitleBar(bar);
    agent->setSystemButton(QWK::WindowAgentBase::Close, closeButton);
    QObject::connect(bar, &QWK::WindowBar::closeRequested, dialog, &QWidget::close);

    // QLayout keeps its "menu bar" above everything else, edge to edge.
    layout->setMenuBar(bar);
#endif
}

void DoseTitleBar::embedInTitle(QMainWindow *window, QWidget *widget)
{
#ifdef Q_OS_MACOS
    Q_UNUSED(window);
    Q_UNUSED(widget);
#else
    auto *bar = window ? qobject_cast<QWK::WindowBar *>(window->menuWidget()) : nullptr;
    QLabel *label = bar ? bar->titleLabel() : nullptr;
    if (!label)
        return;
    // The title's text hides; the label's room hosts the widget, centred.
    label->setProperty("doseHosting", true); // the title text hides (stylesheet rule), children don't
    // Room above and below the field whatever its height (fonts differ), so
    // its border never meets the window's top edge.
    auto *row = new QHBoxLayout(label);
    row->setContentsMargins(12, 6, 12, 6);
    row->addStretch();
    row->addWidget(widget);
    row->addStretch();
    if (auto *agent = window->findChild<QWK::WidgetWindowAgent *>())
        agent->setHitTestVisible(widget, true);
#endif
}
