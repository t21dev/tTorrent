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

#include "dosehealth.h"

#include <QDialog>
#include <QHostInfo>
#include <QLabel>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QPointer>
#include <QRandomGenerator>
#include <QTimer>
#include <QUdpSocket>
#include <QUrl>
#include <QtEndian>

#include "base/bittorrent/infohash.h"
#include "base/bittorrent/torrentdescriptor.h"
#include "base/bittorrent/trackerentry.h"
#include "base/global.h"
#include "base/version.h"
#include "dosepaint.h"

namespace
{
    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseHealth", text);
    }

    // The best answer so far, shown in the label as it improves.
    class Probe final : public QObject
    {
    public:
        Probe(QLabel *label, const QByteArray &hash, const QStringList &trackers)
            : QObject(label)
            , m_label {label}
            , m_hash {hash}
        {
            m_pending = int(trackers.size());
            for (const QString &url : trackers)
            {
                if (url.startsWith(u"udp://", Qt::CaseInsensitive))
                    udp(QUrl(url));
                else
                    http(url);
            }
            QTimer::singleShot(8000, this, [this]
            {
                m_pending = 0;
                show();
            });
            show();
        }

    private:
        void answer(const int seeds, const int peers)
        {
            if ((seeds >= 0) && ((seeds > m_seeds) || ((seeds == m_seeds) && (peers > m_peers))))
            {
                m_seeds = seeds;
                m_peers = peers;
            }
            done();
        }

        void done()
        {
            m_pending = std::max(0, m_pending - 1);
            show();
        }

        void show()
        {
            if (!m_label)
                return;
            QString text;
            QString role = u"faint"_s;
            if (m_seeds >= 0)
            {
                text = tr("%1 seeds · %2 peers").arg(QString::number(m_seeds), QString::number(std::max(0, m_peers)));
                role = (m_seeds == 0) ? u"bad"_s : ((m_seeds < 3) ? u"warn"_s : u"ok"_s);
                if (m_pending > 0)
                    text += u"  …"_s;
            }
            else
            {
                text = (m_pending > 0) ? tr("Asking trackers…") : tr("Trackers didn't answer");
            }
            m_label->setText(text);
            m_label->setToolTip(tr("Counts reported by the torrent's trackers. Peers found through DHT aren't included."));
            const QColor colour = DosePaint::token(role);
            if (colour.isValid())
                m_label->setStyleSheet(u"color: %1;"_s.arg(colour.name()));
        }

        // BEP 48: replace the last "announce" in the path with "scrape".
        void http(const QString &announce)
        {
            const qsizetype at = announce.lastIndexOf(u"/announce");
            if (at < 0)
                return done();
            QString scrape = announce;
            scrape.replace(at, 9, u"/scrape"_s);
            const QByteArray sep = scrape.contains(u'?') ? "&" : "?";
            const QByteArray encoded = QByteArray(scrape.toUtf8() + sep + "info_hash=" + m_hash.toPercentEncoding());
            const QUrl url = QUrl::fromEncoded(encoded);
            QNetworkRequest request {url};
            request.setTransferTimeout(7000);
            // Trackers only ever see qBittorrent, as with libtorrent's announces.
            request.setHeader(QNetworkRequest::UserAgentHeader, QStringLiteral("qBittorrent/" QBT_VERSION_2));
            static auto *network = new QNetworkAccessManager(qApp);
            QNetworkReply *reply = network->get(request);
            connect(reply, &QNetworkReply::finished, this, [this, reply]
            {
                reply->deleteLater();
                const QByteArray body = reply->readAll();
                const auto number = [&body](const QByteArray &key) -> int
                {
                    const qsizetype at = body.indexOf(key);
                    if (at < 0)
                        return -1;
                    const qsizetype start = at + key.size();
                    const qsizetype end = body.indexOf('e', start);
                    bool ok = false;
                    const int value = body.mid(start, end - start).toInt(&ok);
                    return ok ? value : -1;
                };
                if (reply->error() == QNetworkReply::NoError)
                    answer(number("8:completei"), number("10:incompletei"));
                else
                    done();
            });
        }

        // BEP 15: connect, then scrape one info hash.
        void udp(const QUrl &url)
        {
            if (url.host().isEmpty() || (url.port() <= 0))
                return done();
            QHostInfo::lookupHost(url.host(), this, [this, port = url.port()](const QHostInfo &info)
            {
                if (info.addresses().isEmpty())
                    return done();
                auto *socket = new QUdpSocket(this);
                const QHostAddress address = info.addresses().first();
                const quint32 transaction = QRandomGenerator::global()->generate();
                const auto send = [socket, address, port](const QByteArray &packet) { socket->writeDatagram(packet, address, quint16(port)); };
                QByteArray connectPacket(16, 0);
                qToBigEndian<quint64>(0x41727101980ULL, connectPacket.data());
                qToBigEndian<quint32>(0, connectPacket.data() + 8);
                qToBigEndian<quint32>(transaction, connectPacket.data() + 12);
                connect(socket, &QUdpSocket::readyRead, this, [this, socket, send, transaction]
                {
                    while (socket->hasPendingDatagrams())
                    {
                        QByteArray data(int(socket->pendingDatagramSize()), 0);
                        socket->readDatagram(data.data(), data.size());
                        if ((data.size() < 8) || (qFromBigEndian<quint32>(data.constData() + 4) != transaction))
                            continue;
                        const quint32 action = qFromBigEndian<quint32>(data.constData());
                        if ((action == 0) && (data.size() >= 16))
                        {
                            QByteArray scrape(16, 0);
                            scrape.replace(0, 8, data.mid(8, 8));
                            qToBigEndian<quint32>(2, scrape.data() + 8);
                            qToBigEndian<quint32>(transaction, scrape.data() + 12);
                            send(scrape + m_hash);
                        }
                        else if ((action == 2) && (data.size() >= 20))
                        {
                            const int seeds = int(qFromBigEndian<quint32>(data.constData() + 8));
                            const int peers = int(qFromBigEndian<quint32>(data.constData() + 16));
                            socket->deleteLater();
                            return answer(seeds, peers);
                        }
                        else
                        {
                            socket->deleteLater();
                            return done();
                        }
                    }
                });
                send(connectPacket);
            });
        }

        QPointer<QLabel> m_label;
        QByteArray m_hash;
        int m_pending = 0;
        int m_seeds = -1;
        int m_peers = -1;
    };
}

void DoseHealth::check(QDialog *dialog, const BitTorrent::TorrentDescriptor &torrent)
{
    auto *label = dialog ? dialog->findChild<QLabel *>(u"doseHealthData"_s) : nullptr;
    if (!label)
        return;
    for (QObject *old : label->children())
        delete old;
    // A private tracker counts every request on the passkey, and some only
    // allow announces from approved clients, so it is never asked here.
    if (torrent.info() && torrent.info()->isPrivate())
    {
        label->setText(tr("Private tracker: shown after adding"));
        label->setStyleSheet({});
        return;
    }
    QStringList trackers;
    for (const BitTorrent::TrackerEntry &entry : torrent.trackers())
    {
        if ((trackers.size() < 5) && (entry.url.startsWith(u"http", Qt::CaseInsensitive) || entry.url.startsWith(u"udp://", Qt::CaseInsensitive)))
            trackers.append(entry.url);
    }
    if (trackers.isEmpty())
    {
        label->setText(tr("No trackers to ask"));
        label->setStyleSheet({});
        return;
    }
    const QByteArray hash = QByteArray::fromHex(torrent.infoHash().toTorrentID().toString().toLatin1());
    new Probe(label, hash, trackers);
}
