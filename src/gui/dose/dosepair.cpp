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

#include "dosepair.h"

#include <QAction>
#include <QApplication>
#include <QClipboard>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFormLayout>
#include <QHostAddress>
#include <QLabel>
#include <QLineEdit>
#include <QMainWindow>
#include <QMenu>
#include <QNetworkInterface>
#include <QPainter>
#include <QPushButton>
#include <QTimer>
#include <QVBoxLayout>

#include "base/global.h"
#include "base/preferences.h"
#include "base/utils/password.h"
#include "gui/uithememanager.h"
#include "thirdparty/qrcodegen.hpp"

namespace
{
    QString tr(const char *text)
    {
        return QCoreApplication::translate("DosePair", text);
    }

    // This computer's address on the local network (private IPv4 first).
    QString lanAddress()
    {
        QString fallback;
        for (const QNetworkInterface &nic : QNetworkInterface::allInterfaces())
        {
            const auto flags = nic.flags();
            if (!flags.testFlag(QNetworkInterface::IsUp) || !flags.testFlag(QNetworkInterface::IsRunning)
                || flags.testFlag(QNetworkInterface::IsLoopBack))
            {
                continue;
            }
            if ((nic.type() == QNetworkInterface::Virtual) || nic.humanReadableName().contains(u"vEthernet", Qt::CaseInsensitive))
                continue;
            for (const QNetworkAddressEntry &entry : nic.addressEntries())
            {
                const QHostAddress ip = entry.ip();
                if (ip.protocol() != QAbstractSocket::IPv4Protocol)
                    continue;
                if (ip.isPrivateUse())
                    return ip.toString();
                if (fallback.isEmpty() && !ip.isLinkLocal())
                    fallback = ip.toString();
            }
        }
        return fallback;
    }

    QString webUIUrl()
    {
        const Preferences *pref = Preferences::instance();
        QString host = pref->getWebUIAddress().trimmed();
        if (host.isEmpty() || (host == u"*") || (host == u"0.0.0.0") || (host == u"::"))
            host = lanAddress();
        if (host.isEmpty())
            return {};
        return u"%1://%2:%3/"_s.arg((pref->isWebUIHttpsEnabled() ? u"https"_s : u"http"_s), host, QString::number(pref->getWebUIPort()));
    }

    // Dark modules on white with the standard quiet zone, so any phone camera reads it.
    QPixmap qrPixmap(const QString &text, const int size)
    {
        const qrcodegen::QrCode code = qrcodegen::QrCode::encodeText(text.toUtf8().constData(), qrcodegen::QrCode::Ecc::MEDIUM);
        const int modules = code.getSize() + 8;
        const qreal dpr = qApp->devicePixelRatio();
        QPixmap pixmap {QSize(size, size) * dpr};
        pixmap.setDevicePixelRatio(dpr);
        pixmap.fill(Qt::white);
        QPainter painter {&pixmap};
        const qreal cell = qreal(size) / modules;
        painter.setPen(Qt::NoPen);
        painter.setBrush(QColor(0x14, 0x17, 0x16));
        for (int y = 0; y < code.getSize(); ++y)
        {
            for (int x = 0; x < code.getSize(); ++x)
            {
                if (code.getModule(x, y))
                    painter.drawRect(QRectF((x + 4) * cell, (y + 4) * cell, cell + 0.02, cell + 0.02));
            }
        }
        return pixmap;
    }

    class PairDialog final : public QDialog
    {
    public:
        explicit PairDialog(QWidget *parent)
            : QDialog(parent)
        {
            setWindowTitle(tr("Pair a phone"));
            setAttribute(Qt::WA_DeleteOnClose);
            m_column = new QVBoxLayout(this);
            m_column->setContentsMargins(20, 8, 20, 16);
            m_column->setSpacing(10);
            build();
        }

    private:
        void clear()
        {
            while (QLayoutItem *item = m_column->takeAt(0))
            {
                if (item->widget())
                    item->widget()->deleteLater();
                delete item;
            }
        }

        void build()
        {
            const Preferences *pref = Preferences::instance();
            if (!pref->isWebUIEnabled() || pref->getWebUIPassword().isEmpty())
                buildSetup();
            else
                buildCode();
            adjustSize();
        }

        // Web UI off: one step to turn it on, with a login.
        void buildSetup()
        {
            auto *intro = new QLabel(tr("Your phone opens tTorrent through the Web UI. Turn it on with a login, then scan the code with the phone's camera."));
            intro->setWordWrap(true);
            intro->setMaximumWidth(380);
            m_column->addWidget(intro);
            auto *form = new QFormLayout;
            const QString saved = Preferences::instance()->getWebUIUsername();
            auto *user = new QLineEdit(saved.isEmpty() ? u"admin"_s : saved);
            auto *password = new QLineEdit;
            password->setEchoMode(QLineEdit::Password);
            password->setPlaceholderText(tr("At least 6 characters"));
            form->addRow(tr("Username:"), user);
            form->addRow(tr("Password:"), password);
            auto *holder = new QWidget;
            holder->setLayout(form);
            m_column->addWidget(holder);
            auto *buttons = new QDialogButtonBox;
            QPushButton *go = buttons->addButton(tr("Turn on and show code"), QDialogButtonBox::AcceptRole);
            go->setDefault(true);
            go->setEnabled(false);
            buttons->addButton(QDialogButtonBox::Cancel);
            connect(password, &QLineEdit::textChanged, go, [go, user, password]
            {
                go->setEnabled((password->text().size() >= 6) && !user->text().trimmed().isEmpty());
            });
            connect(buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
            connect(go, &QPushButton::clicked, this, [this, user, password]
            {
                Preferences *pref = Preferences::instance();
                pref->setWebUIUsername(user->text().trimmed());
                pref->setWebUIPassword(Utils::Password::PBKDF2::generate(password->text()));
                pref->setWebUIEnabled(true);
                pref->apply();
                clear();
                QTimer::singleShot(0, this, [this] { build(); });
            });
            m_column->addWidget(buttons);
        }

        void buildCode()
        {
            const QString url = webUIUrl();
            if (url.isEmpty())
            {
                m_column->addWidget(new QLabel(tr("This computer has no local network address. Connect it to Wi-Fi or a network cable first.")));
                return;
            }
            auto *code = new QLabel;
            code->setPixmap(qrPixmap(url, 220));
            code->setAlignment(Qt::AlignCenter);
            code->setObjectName(u"doseQrCode"_s);
            // White behind the code in every theme: phone cameras need dark on light.
            code->setStyleSheet(u"background: white; border-radius: 12px; padding: 6px;"_s);
            m_column->addWidget(code, 0, Qt::AlignHCenter);
            auto *link = new QLabel(url);
            link->setTextInteractionFlags(Qt::TextSelectableByMouse);
            link->setAlignment(Qt::AlignCenter);
            link->setObjectName(u"dosePairLink"_s);
            m_column->addWidget(link);
            auto *note = new QLabel(tr("Scan with the phone's camera. The phone must be on the same network as this computer. Sign in with your Web UI login."));
            note->setWordWrap(true);
            note->setMaximumWidth(320);
            note->setAlignment(Qt::AlignCenter);
            note->setObjectName(u"doseMuted"_s);
            m_column->addWidget(note, 0, Qt::AlignHCenter);
            auto *buttons = new QDialogButtonBox;
            QPushButton *copy = buttons->addButton(tr("Copy link"), QDialogButtonBox::ActionRole);
            copy->setAutoDefault(false);
            QPushButton *done = buttons->addButton(QDialogButtonBox::Close);
            done->setDefault(true);
            connect(copy, &QPushButton::clicked, this, [copy, url]
            {
                QApplication::clipboard()->setText(url);
                copy->setText(tr("Copied"));
            });
            connect(done, &QPushButton::clicked, this, &QDialog::accept);
            m_column->addWidget(buttons);
        }

        QVBoxLayout *m_column = nullptr;
    };
}

void DosePair::install(QMainWindow *window)
{
    auto *tools = window ? window->findChild<QMenu *>(u"menuOptions"_s) : nullptr;
    if (!tools)
        return;
    auto *pair = new QAction(UIThemeManager::instance()->getIcon(u"preferences-webui"_s), tr("Pair a phone…"), window);
    pair->setObjectName(u"doseActionPair"_s);
    QObject::connect(pair, &QAction::triggered, window, [window] { show(window); });
    QAction *options = window->findChild<QAction *>(u"actionOptions"_s);
    tools->insertAction((options && tools->actions().contains(options)) ? options : nullptr, pair);
}

void DosePair::show(QWidget *parent)
{
    auto *dialog = new PairDialog(parent);
    dialog->open();
}
