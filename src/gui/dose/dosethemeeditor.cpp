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

#include "dosethemeeditor.h"

#include <QBoxLayout>
#include <QColorDialog>
#include <QComboBox>
#include <QFile>
#include <QFileDialog>
#include <QGridLayout>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QLineEdit>
#include <QPainter>
#include <QPainterPath>
#include <QPushButton>
#include <QRegularExpressionValidator>
#include <QScreen>
#include <QScrollArea>
#include <QToolButton>

#include "base/global.h"
#include "gui/uithememanager.h"
#include "dosetheme.h"

namespace
{
    struct TokenInfo
    {
        const char *key;
        const char *name;
        const char *hint;
        double needs;  // contrast against bg; 0 = none
    };

    const QList<std::pair<const char *, QList<TokenInfo>>> &groups()
    {
        static const QList<std::pair<const char *, QList<TokenInfo>>> list {
            {QT_TRANSLATE_NOOP("DoseThemeEditor", "Surfaces"), {
                {"bg", QT_TRANSLATE_NOOP("DoseThemeEditor", "Background"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Window and lists"), 0},
                {"panel", QT_TRANSLATE_NOOP("DoseThemeEditor", "Panel"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Sidebar, status bar, fields"), 0},
                {"raised", QT_TRANSLATE_NOOP("DoseThemeEditor", "Raised"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Menus and buttons"), 0},
                {"hover", QT_TRANSLATE_NOOP("DoseThemeEditor", "Hover"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Under the pointer"), 0},
                {"line", QT_TRANSLATE_NOOP("DoseThemeEditor", "Lines"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Hairlines between panes"), 0}}},
            {QT_TRANSLATE_NOOP("DoseThemeEditor", "Text"), {
                {"text", QT_TRANSLATE_NOOP("DoseThemeEditor", "Text"), "", 7.0},
                {"muted", QT_TRANSLATE_NOOP("DoseThemeEditor", "Secondary text"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Headers and icons"), 4.5},
                {"faint", QT_TRANSLATE_NOOP("DoseThemeEditor", "Faint text"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Counts and hints"), 4.5},
                {"ring", QT_TRANSLATE_NOOP("DoseThemeEditor", "Disabled"), "", 0}}},
            {QT_TRANSLATE_NOOP("DoseThemeEditor", "Accent and states"), {
                {"accent", QT_TRANSLATE_NOOP("DoseThemeEditor", "Accent"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Selection and progress"), 3.0},
                {"ok", QT_TRANSLATE_NOOP("DoseThemeEditor", "Good"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Seeding, completed"), 3.0},
                {"warn", QT_TRANSLATE_NOOP("DoseThemeEditor", "Warning"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Checking, stalled"), 3.0},
                {"bad", QT_TRANSLATE_NOOP("DoseThemeEditor", "Error"), QT_TRANSLATE_NOOP("DoseThemeEditor", "Errors and Remove"), 3.0}}},
        };
        return list;
    }

    QString trE(const char *text)
    {
        return QCoreApplication::translate("DoseThemeEditor", text);
    }

    // A small picture of the window in the draft's colours: title bar,
    // sidebar, four torrents (one per state), buttons, a tooltip, status bar.
    class Preview final : public QWidget
    {
    public:
        explicit Preview(QWidget *parent) : QWidget(parent)
        {
            setMinimumSize(340, 300);
        }

        void setTheme(const QJsonObject &theme)
        {
            // The draft's own Accent colour, not the app's accent choice.
            m_source = std::make_unique<DoseThemeSource>(theme, u"theme"_s, u"compact"_s);
            update();
        }

    protected:
        void paintEvent(QPaintEvent *) override
        {
            if (!m_source)
                return;
            const auto t = [this](const char *name) { return m_source->token(QString::fromLatin1(name)); };
            QPainter p {this};
            p.setRenderHint(QPainter::Antialiasing);
            const QRectF r = QRectF(rect()).adjusted(0.5, 0.5, -0.5, -0.5);
            QPainterPath frame;
            frame.addRoundedRect(r, 14, 14);
            p.setClipPath(frame);
            p.fillRect(r, t("bg"));

            QFont ui = font();
            ui.setPixelSize(12);
            QFont small = ui;
            small.setPixelSize(11);
            p.setFont(ui);

            // Title bar.
            const QRectF title {r.left(), r.top(), r.width(), 30};
            p.fillRect(title, t("panel"));
            p.setPen(t("line"));
            p.drawLine(title.bottomLeft(), title.bottomRight());
            p.setPen(Qt::NoPen);
            p.setBrush(t("text"));
            p.drawEllipse(QPointF(title.left() + 16, title.center().y()), 7, 7);
            p.setPen(t("muted"));
            p.drawText(title.adjusted(32, 0, -10, 0), Qt::AlignVCenter | Qt::AlignLeft, tr("File   Edit   View"));
            p.setPen(t("faint"));
            p.drawText(title.adjusted(0, 0, -10, 0), Qt::AlignVCenter | Qt::AlignRight, u"tTorrent"_s);

            // Status bar.
            const QRectF status {r.left(), r.bottom() - 26, r.width(), 26};
            p.fillRect(status, t("panel"));
            p.setPen(t("line"));
            p.drawLine(status.topLeft(), status.topRight());
            p.setFont(small);
            p.setPen(t("muted"));
            p.drawText(status.adjusted(10, 0, 0, 0), Qt::AlignVCenter | Qt::AlignLeft, u"192 GB free"_s);
            p.setPen(t("ok"));
            p.drawText(status.adjusted(0, 0, -10, 0), Qt::AlignVCenter | Qt::AlignRight, u"↑ 312 KB/s"_s);
            p.setPen(t("accent"));
            p.drawText(status.adjusted(0, 0, -90, 0), Qt::AlignVCenter | Qt::AlignRight, u"↓ 8.4 MB/s"_s);

            // Sidebar.
            const QRectF side {r.left(), title.bottom(), 96, status.top() - title.bottom()};
            p.fillRect(side, t("panel"));
            p.setPen(t("line"));
            p.drawLine(side.topRight(), side.bottomRight());
            p.setFont(ui);
            const QStringList filters {tr("All 12"), tr("Downloading"), tr("Seeding"), tr("Errored")};
            for (int i = 0; i < filters.size(); ++i)
            {
                const QRectF item {side.left() + 6, side.top() + 8 + (i * 26), side.width() - 12, 22};
                if (i == 0)
                {
                    p.setPen(Qt::NoPen);
                    p.setBrush(t("select"));
                    p.drawRoundedRect(item, 7, 7);
                }
                p.setPen((i == 0) ? t("text") : t("muted"));
                p.drawText(item.adjusted(8, 0, 0, 0), Qt::AlignVCenter | Qt::AlignLeft, filters[i]);
            }

            // Torrents.
            const struct { QString name; const char *state; QString word; qreal done; } rows[] {
                {u"ubuntu-24.04.3-desktop.iso"_s, "accent", tr("Downloading"), 0.62},
                {u"Sintel (2010) 1080p.mkv"_s, "ok", tr("Seeding"), 1.0},
                {u"linuxmint-22.2.iso"_s, "warn", tr("Checking"), 0.37},
                {u"Cosmos Laundromat.mkv"_s, "bad", tr("Missing files"), 0.88},
            };
            const QRectF list {side.right() + 1, title.bottom(), r.right() - side.right() - 1, status.top() - title.bottom()};
            qreal y = list.top() + 4;
            for (int i = 0; i < 4; ++i)
            {
                const QRectF row {list.left(), y, list.width(), 40};
                if (i == 0)
                {
                    p.fillRect(row, t("select"));
                    p.fillRect(QRectF(row.left(), row.top(), 3, row.height()), t("accent"));
                }
                p.setFont(ui);
                p.setPen(t("text"));
                p.drawText(row.adjusted(10, 4, -90, -18), Qt::AlignVCenter | Qt::AlignLeft
                    , QFontMetrics(ui).elidedText(rows[i].name, Qt::ElideRight, int(row.width() - 110)));
                p.setFont(small);
                p.setPen(t(rows[i].state));
                p.drawText(row.adjusted(0, 4, -10, -18), Qt::AlignVCenter | Qt::AlignRight, rows[i].word);
                const QRectF track {row.left() + 10, row.bottom() - 11, row.width() - 20, 4};
                p.setPen(Qt::NoPen);
                p.setBrush(t("track"));
                p.drawRoundedRect(track, 2, 2);
                p.setBrush(t(rows[i].state));
                p.drawRoundedRect(QRectF(track.left(), track.top(), track.width() * rows[i].done, track.height()), 2, 2);
                p.setPen(t("rowRule"));
                p.drawLine(row.bottomLeft(), row.bottomRight());
                y += 40;
            }

            // Buttons and a tooltip.
            p.setFont(ui);
            const QRectF primary {list.left() + 10, y + 12, 96, 26};
            p.setPen(Qt::NoPen);
            p.setBrush(t("accent"));
            p.drawRoundedRect(primary, 8, 8);
            p.setPen(t("onAccent"));
            QFont bold = ui;
            bold.setWeight(QFont::DemiBold);
            p.setFont(bold);
            p.drawText(primary, Qt::AlignCenter, tr("Add torrent"));
            const QRectF secondary {primary.right() + 8, primary.top(), 70, 26};
            p.setPen(t("line"));
            p.setBrush(t("raised"));
            p.drawRoundedRect(secondary, 8, 8);
            p.setFont(ui);
            p.setPen(t("text"));
            p.drawText(secondary, Qt::AlignCenter, tr("Cancel"));
            const QRectF tip {secondary.right() + 8, primary.top() + 2, list.right() - secondary.right() - 18, 22};
            if (tip.width() > 40)
            {
                p.setPen(t("line"));
                p.setBrush(t("raised"));
                p.drawRoundedRect(tip, 7, 7);
                p.setFont(small);
                p.setPen(t("muted"));
                p.drawText(tip, Qt::AlignCenter, tr("Tooltip · Ctrl+O"));
            }

            p.setClipping(false);
            p.setPen(QPen(t("line"), 1));
            p.setBrush(Qt::NoBrush);
            p.drawRoundedRect(r, 14, 14);
        }

    private:
        std::unique_ptr<DoseThemeSource> m_source;
    };
}

DoseThemeEditor::DoseThemeEditor(const QString &startFrom, QWidget *parent)
    : QDialog(parent)
{
    setWindowTitle(tr("Create a theme"));
    setObjectName(u"doseThemeEditor"_s);

    auto *root = new QVBoxLayout(this);
    root->setContentsMargins(18, 14, 18, 14);
    root->setSpacing(12);

    // Name and starting point.
    auto *top = new QHBoxLayout;
    top->setSpacing(10);
    top->addWidget(new QLabel(tr("Name")));
    m_name = new QLineEdit(tr("My theme"));
    top->addWidget(m_name, 1);
    top->addWidget(new QLabel(tr("Start from")));
    m_base = new QComboBox;
    for (const DoseThemeSource::Choice &c : DoseThemeSource::themes())
    {
        if (c.id != u"system")
            m_base->addItem(c.name, c.id);
    }
    top->addWidget(m_base);
    root->addLayout(top);

    // Tiles (scrolling) beside the preview (fixed).
    auto *body = new QHBoxLayout;
    body->setSpacing(18);
    auto *tilesHost = new QWidget;
    auto *tilesCol = new QVBoxLayout(tilesHost);
    tilesCol->setContentsMargins(0, 0, 6, 0);
    tilesCol->setSpacing(12);
    const QRegularExpression hexPattern {u"#?[0-9A-Fa-f]{0,6}"_s};
    for (const auto &[group, tokens] : groups())
    {
        auto *heading = new QLabel(trE(group));
        heading->setObjectName(u"doseEditorGroup"_s);
        tilesCol->addWidget(heading);
        auto *grid = new QGridLayout;
        grid->setHorizontalSpacing(8);
        grid->setVerticalSpacing(8);
        int i = 0;
        for (const TokenInfo &info : tokens)
        {
            const QString key = QString::fromLatin1(info.key);
            auto *tile = new QFrame;
            tile->setObjectName(u"doseEditorTile"_s);
            if (*info.hint)
                tile->setToolTip(trE(info.hint));
            auto *row = new QHBoxLayout(tile);
            row->setContentsMargins(6, 6, 8, 6);
            row->setSpacing(9);
            Tile t;
            t.swatch = new QToolButton;
            t.swatch->setFixedSize(30, 30);
            t.swatch->setCursor(Qt::PointingHandCursor);
            t.swatch->setToolTip(tr("Pick a colour"));
            auto *names = new QVBoxLayout;
            names->setSpacing(0);
            auto *name = new QLabel(trE(info.name));
            t.ratio = new QLabel;
            t.ratio->setObjectName(u"doseEditorRatio"_s);
            t.ratio->setProperty("needs", info.needs);
            names->addWidget(name);
            names->addWidget(t.ratio);
            t.hex = new QLineEdit;
            t.hex->setFixedWidth(100);
            t.hex->setStyleSheet(u"padding: 0 6px; font-family: \"DM Mono\";"_s);
            t.hex->setMaxLength(7);
            t.hex->setValidator(new QRegularExpressionValidator(hexPattern, t.hex));
            row->addWidget(t.swatch);
            row->addLayout(names, 1);
            row->addWidget(t.hex);
            connect(t.swatch, &QToolButton::clicked, this, [this, key]
            {
                const QColor picked = QColorDialog::getColor(m_colours.value(key), this, tr("Pick a colour"));
                if (picked.isValid())
                    setColour(key, picked);
            });
            connect(t.hex, &QLineEdit::editingFinished, this, [this, key, hex = t.hex]
            {
                QString text = hex->text().trimmed();
                if (!text.startsWith(u'#'))
                    text.prepend(u'#');
                const QColor c {text};
                if ((text.size() == 7) && c.isValid())
                    setColour(key, c);
                else
                    hex->setText(m_colours.value(key).name().toUpper());
            });
            m_tiles.insert(key, t);
            grid->addWidget(tile, i / 2, i % 2);
            ++i;
        }
        grid->setColumnStretch(0, 1);
        grid->setColumnStretch(1, 1);
        tilesCol->addLayout(grid);
    }
    tilesCol->addStretch();
    auto *scroll = new QScrollArea;
    scroll->setWidget(tilesHost);
    scroll->setWidgetResizable(true);
    scroll->setFrameShape(QFrame::NoFrame);
    scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    body->addWidget(scroll, 1);

    auto *previewCol = new QVBoxLayout;
    previewCol->setSpacing(8);
    auto *previewLabel = new QLabel(tr("Preview"));
    previewLabel->setObjectName(u"doseEditorGroup"_s);
    m_preview = new Preview(this);
    auto *note = new QLabel(tr("Contrast is checked as you go: text needs 7:1, secondary text 4.5:1, "
        "the accent 3:1. A colour that fails is marked, and can still be saved."));
    note->setWordWrap(true);
    note->setObjectName(u"doseEditorNote"_s);
    note->setMaximumWidth(340);
    previewCol->addWidget(previewLabel);
    previewCol->addWidget(m_preview);
    previewCol->addWidget(note);
    previewCol->addStretch();
    body->addLayout(previewCol);
    root->addLayout(body, 1);

    // Footer.
    auto *footer = new QHBoxLayout;
    auto *importButton = new QPushButton(tr("Import…"));
    auto *exportButton = new QPushButton(tr("Export…"));
    auto *cancel = new QPushButton(tr("Cancel"));
    auto *saveButton = new QPushButton(tr("Save theme"));
    for (QPushButton *b : {importButton, exportButton, cancel})
        b->setAutoDefault(false);
    saveButton->setDefault(true);
    footer->addWidget(importButton);
    footer->addWidget(exportButton);
    footer->addStretch();
    footer->addWidget(cancel);
    footer->addWidget(saveButton);
    root->addLayout(footer);
    connect(importButton, &QPushButton::clicked, this, &DoseThemeEditor::importTheme);
    connect(exportButton, &QPushButton::clicked, this, &DoseThemeEditor::exportTheme);
    connect(cancel, &QPushButton::clicked, this, &QDialog::reject);
    connect(saveButton, &QPushButton::clicked, this, &DoseThemeEditor::save);

    setStyleSheet(uR"(
QFrame#doseEditorTile { border: 1px solid palette(mid); border-radius: 10px; background: palette(midlight); }
QLabel#doseEditorGroup { color: palette(dark); font-weight: 600; }
QLabel#doseEditorNote { color: palette(dark); }
)"_s);

    connect(m_base, &QComboBox::activated, this, [this](int) { loadFrom(m_base->currentData().toString()); });
    const int start = std::max(0, m_base->findData(startFrom));
    m_base->setCurrentIndex(start);
    loadFrom(m_base->currentData().toString());

    // Never taller than most of the screen: the tiles scroll, the preview stays.
    const QRect screen = (parent && parent->screen()) ? parent->screen()->availableGeometry() : QRect(0, 0, 1280, 800);
    resize(std::min(920, int(screen.width() * 0.92)), std::min(640, int(screen.height() * 0.85)));
    setMaximumHeight(int(screen.height() * 0.9));
}

QString DoseThemeEditor::savedThemeId() const
{
    return m_savedId;
}

void DoseThemeEditor::loadFrom(const QString &themeId)
{
    const QJsonObject theme = DoseThemeSource::themeJson(themeId);
    m_dark = (theme.value(u"appearance"_s).toString() == u"dark");
    const QJsonObject colours = theme.value(u"colors"_s).toObject();
    for (const QString &key : DoseThemeSource::baseTokens())
        m_colours[key] = QColor(colours.value(key).toString());
    refresh();
}

void DoseThemeEditor::setColour(const QString &token, const QColor &colour)
{
    m_colours[token] = colour;
    if (token == u"bg")
    {
        // A dark background makes a dark theme (icons, native parts).
        m_dark = (colour.lightnessF() < 0.45);
    }
    refresh();
}

void DoseThemeEditor::refresh()
{
    const QColor bg = m_colours.value(u"bg"_s);
    for (auto it = m_tiles.cbegin(); it != m_tiles.cend(); ++it)
    {
        const QColor c = m_colours.value(it.key());
        it->swatch->setStyleSheet(u"QToolButton { background: %1; border: 1px solid palette(mid); border-radius: 8px; }"_s.arg(c.name()));
        if (!it->hex->hasFocus())
            it->hex->setText(c.name().toUpper());
        const double needs = it->ratio->property("needs").toDouble();
        if (needs > 0)
        {
            const double r = DoseThemeSource::contrastRatio(c, bg);
            it->ratio->setText(u"%1:1"_s.arg(r, 0, 'f', 1));
            const QColor mark = UIThemeManager::instance()->doseToken((r >= needs) ? u"ok"_s : u"bad"_s);
            it->ratio->setStyleSheet(u"font-family: \"DM Mono\"; font-size: 11px; color: %1;"_s.arg(mark.name()));
        }
        else
        {
            it->ratio->clear();
        }
    }

    QJsonObject tokens;
    for (const QString &key : DoseThemeSource::baseTokens())
        tokens.insert(key, m_colours.value(key).name());
    static_cast<Preview *>(m_preview)->setTheme(QJsonObject {
        {u"id"_s, u"draft"_s}, {u"appearance"_s, m_dark ? u"dark"_s : u"light"_s}, {u"colors"_s, tokens}});
}

void DoseThemeEditor::save()
{
    m_savedId = DoseThemeSource::saveUserTheme(m_name->text(), m_dark, m_colours);
    if (!m_savedId.isEmpty())
        accept();
}

void DoseThemeEditor::importTheme()
{
    const QString path = QFileDialog::getOpenFileName(this, tr("Import a theme"), {}, tr("tTorrent theme (*.json)"));
    if (path.isEmpty())
        return;
    QFile file {path};
    if (!file.open(QIODevice::ReadOnly))
        return;
    const QJsonObject theme = QJsonDocument::fromJson(file.readAll()).object();
    const QJsonObject colours = theme.value(u"colors"_s).toObject();
    for (const QString &key : DoseThemeSource::baseTokens())
    {
        const QColor c {colours.value(key).toString()};
        if (c.isValid())
            m_colours[key] = c;
    }
    m_dark = (theme.value(u"appearance"_s).toString() == u"dark");
    if (const QString name = theme.value(u"name"_s).toString(); !name.isEmpty())
        m_name->setText(name);
    refresh();
}

void DoseThemeEditor::exportTheme()
{
    const QString path = QFileDialog::getSaveFileName(this, tr("Export this theme"), m_name->text() + u".json", tr("tTorrent theme (*.json)"));
    if (path.isEmpty())
        return;
    QJsonObject tokens;
    for (const QString &key : DoseThemeSource::baseTokens())
        tokens.insert(key, m_colours.value(key).name().toUpper());
    QFile file {path};
    if (file.open(QIODevice::WriteOnly | QIODevice::Truncate))
    {
        file.write(QJsonDocument(QJsonObject {{u"name"_s, m_name->text()}, {u"appearance"_s, m_dark ? u"dark"_s : u"light"_s}
            , {u"colors"_s, tokens}}).toJson(QJsonDocument::Indented));
    }
}
