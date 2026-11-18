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

#include "dosedialogs.h"

#include <QAbstractButton>
#include <QTreeView>
#include <QHeaderView>
#include <QSvgRenderer>
#include <QPainter>
#include <QToolBar>
#include <QMainWindow>
#include <QApplication>
#include <QBoxLayout>
#include <QButtonGroup>
#include <QCheckBox>
#include <QComboBox>
#include <QGridLayout>
#include <QDialog>
#include <QGroupBox>
#include <QHash>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPushButton>
#include <QRegularExpression>
#include <QRegularExpressionValidator>
#include <QScrollArea>
#include <QSplitter>
#include <QStackedWidget>
#include <QDialogButtonBox>
#include <QTextDocumentFragment>
#include <QTimer>
#include <QToolButton>
#include <QTreeWidget>
#include <QTreeWidgetItemIterator>

#include <memory>
#include <utility>

#include "base/settingsstorage.h"
#include "gui/uithememanager.h"
#include "doseimport.h"
#include "doseautomation.h"
#include "dosemcp.h"
#include "doseupdate.h"
#include "dosepaint.h"
#include "dosetheme.h"
#include "dosethemeeditor.h"
#include "dosetoolbar.h"
#include "dosetrim.h"

namespace
{
    const char OPTIONS_STYLE[] = R"(
QWidget#doseSidebar {
    background: palette(window);
    border-right: 1px solid palette(mid);
}
QLabel#doseSettingsTitle {
    font-size: 17px;
    font-weight: 700;
    padding: 0 6px 4px 6px;
}
QWidget#doseSidebar QListWidget {
    background: transparent;
    border: none;
}
QWidget#doseSidebar QListWidget::item {
    padding: 4px 8px;
    margin: 1px 0;
    border: none;
    border-radius: 8px;
}
QWidget#doseSidebar QListWidget::item:selected {
    background: palette(highlight);
    color: palette(highlighted-text);
}
QWidget#doseSidebar QListWidget::item:hover:!selected {
    background: palette(alternate-base);
}
QLabel#dosePageTitle {
    font-size: 16px;
    font-weight: 700;
    padding: 6px 24px 10px 24px;
    border-bottom: 1px solid palette(mid);
}
QLabel#doseNoResults {
    color: palette(dark);
    padding: 24px;
}
)";

    const char ADD_STYLE[] = R"(
QWidget#doseAddHeader {
    border-bottom: 1px solid palette(mid);
}
QLabel#doseAddTitle {
    font-size: 17px;
    font-weight: 700;
}
QLabel#doseAddName {
    font-size: 15px;
    font-weight: 600;
}
QWidget#doseAddHeader QLabel[doseRole="key"] {
    color: palette(dark);
}
QToolButton#doseInfoToggle, QToolButton#doseInfoToggle:checked {
    border: none;
    background: transparent;
    padding: 6px 8px 6px 2px;
    color: palette(window-text);
    font-weight: 600;
}
QToolButton#doseInfoToggle:hover { color: palette(link); }
QGroupBox#doseInfoBox { border: none; margin-top: 0; padding: 0 0 0 22px; }
)";

    // A small chevron, right (closed) or down (open), in the text colour.
    QIcon chevron(const bool open, const QColor &ink)
    {
        const QString svg = QStringLiteral("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%1' "
            "stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><path d='%2'/></svg>")
            .arg(ink.name(), open ? QStringLiteral("m6 9 6 6 6-6") : QStringLiteral("m9 18 6-6-6-6"));
        QSvgRenderer renderer {svg.toUtf8()};
        QIcon icon;
        for (const qreal scale : {1.0, 1.5, 2.0})
        {
            QPixmap pixmap {QSize(14, 14) * scale};
            pixmap.setDevicePixelRatio(scale);
            pixmap.fill(Qt::transparent);
            QPainter painter {&pixmap};
            renderer.render(&painter, QRectF(0, 0, 14, 14));
            icon.addPixmap(pixmap);
        }
        return icon;
    }

    // One icon per Options section, drawn in our line style: grey normally,
    // in the accent when the section is selected.
    QIcon sectionIcon(const QString &page)
    {
        static const QHash<QString, QString> paths {
            {QStringLiteral("tabBehaviorPage"), QStringLiteral("<path d='M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M2 14h4M10 8h4M18 16h4'/>")},
            {QStringLiteral("tabDownloadsPage"), QStringLiteral("<path d='M12 15V3'/><path d='m7 10 5 5 5-5'/><path d='M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4'/>")},
            {QStringLiteral("tabConnectionPage"), QStringLiteral("<path d='M12 22v-5'/><path d='M9 8V2M15 8V2'/><path d='M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z'/>")},
            {QStringLiteral("tabSpeedPage"), QStringLiteral("<path d='m12 14 4-4'/><path d='M3.34 19a10 10 0 1 1 17.32 0'/>")},
            {QStringLiteral("tabBitTorrentPage"), QStringLiteral("<circle cx='12' cy='5' r='2.5'/><circle cx='5' cy='18' r='2.5'/><circle cx='19' cy='18' r='2.5'/><path d='M12 7.5v3.5M10.5 12.5 6.5 16M13.5 12.5l4 3.5'/>")},
            {QStringLiteral("tabSearchPage"), QStringLiteral("<circle cx='11' cy='11' r='7'/><path d='m21 21-4.3-4.3'/>")},
            {QStringLiteral("tabRSSPage"), QStringLiteral("<path d='M4 11a9 9 0 0 1 9 9'/><path d='M4 4a16 16 0 0 1 16 16'/><circle cx='5' cy='19' r='1'/>")},
            {QStringLiteral("tabWebUIPage"), QStringLiteral("<rect x='2' y='3' width='20' height='14' rx='2'/><path d='M8 21h8M12 17v4'/>")},
            {QStringLiteral("tabAdvancedPage"), QStringLiteral("<rect x='5' y='5' width='14' height='14' rx='2'/><rect x='9' y='9' width='6' height='6' rx='1'/><path d='M9 2v3M15 2v3M9 19v3M15 19v3M2 9h3M2 15h3M19 9h3M19 15h3'/>")},
        };
        const QString body = paths.value(page);
        if (body.isEmpty())
            return {};
        const auto draw = [&body](const QColor &colour)
        {
            const QString svg = QStringLiteral("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%1' "
                "stroke-width='1.8' stroke-linecap='round' stroke-linejoin='round'>%2</svg>").arg(colour.name(), body);
            QSvgRenderer renderer {svg.toUtf8()};
            QList<QPixmap> list;
            for (const qreal scale : {1.0, 1.5, 2.0})
            {
                QPixmap pixmap {QSize(16, 16) * scale};
                pixmap.setDevicePixelRatio(scale);
                pixmap.fill(Qt::transparent);
                QPainter painter {&pixmap};
                renderer.render(&painter, QRectF(0, 0, 16, 16));
                list.append(pixmap);
            }
            return list;
        };
        const UIThemeManager *theme = UIThemeManager::instance();
        QIcon icon;
        for (const QPixmap &p : draw(theme->doseToken(QStringLiteral("muted"))))
            icon.addPixmap(p, QIcon::Normal);
        for (const QPixmap &p : draw(theme->doseToken(QStringLiteral("accent"))))
            icon.addPixmap(p, QIcon::Selected);
        return icon;
    }

    // Text a reader would search for: labels, buttons, group titles,
    // advanced settings' names. Accelerators and markup removed.
    QString searchableText(const QWidget *page)
    {
        static const QRegularExpression tags {QStringLiteral("<[^>]*>")};
        QStringList parts;
        const auto clean = [](QString s)
        {
            s.remove(tags);
            s.remove(QLatin1Char('&'));
            return s;
        };
        for (const QLabel *l : page->findChildren<QLabel *>())
            parts << clean(l->text());
        for (const QAbstractButton *b : page->findChildren<QAbstractButton *>())
            parts << clean(b->text());
        for (const QGroupBox *g : page->findChildren<QGroupBox *>())
            parts << clean(g->title());
        for (const QTreeWidget *tree : page->findChildren<QTreeWidget *>())
        {
            for (QTreeWidgetItemIterator it {const_cast<QTreeWidget *>(tree)}; *it; ++it)
                parts << (*it)->text(0);
        }
        return parts.join(QLatin1Char('\n')).toLower();
    }

    // Behavior › Look, on Kitab's model: theme, accent (preset or any hex,
    // adjusted for contrast), density, all live; interface size at next start.
    void addLookGroup(QWidget *behaviorPage)
    {
        if (!behaviorPage)
            return;
        // The page's first group box sits in the layout we add to.
        QGroupBox *first = behaviorPage->findChild<QGroupBox *>();
        auto *pageLayout = first ? qobject_cast<QBoxLayout *>(first->parentWidget()->layout()) : nullptr;
        if (!pageLayout)
            return;

        UIThemeManager *manager = UIThemeManager::instance();
        const bool dose = manager->usesDoseTheme();
        // qBittorrent's own colour scheme row would fight the theme picker.
        if (dose)
        {
            for (const char *name : {"labelColorScheme", "comboColorScheme"})
            {
                if (auto *w = behaviorPage->window()->findChild<QWidget *>(QLatin1String(name)))
                    w->hide();
            }
        }

        auto *group = new QGroupBox(QDialog::tr("Look"));
        group->setObjectName(QStringLiteral("doseLook"));
        auto *form = new QGridLayout(group);
        form->setHorizontalSpacing(16);
        form->setVerticalSpacing(10);
        int row = 0;

        auto *theme = new QComboBox;
        theme->setMaxVisibleItems(20);
        const auto fillThemes = [theme, manager]
        {
            theme->clear();
            bool mine = false;
            for (const DoseThemeSource::Choice &c : DoseThemeSource::themes())
            {
                const bool user = c.id.startsWith(u"user-");
                if ((c.id == u"light") || (c.id == u"dark") || (user && !mine))
                    theme->insertSeparator(theme->count());
                mine = mine || user;
                theme->addItem(c.name, c.id);
            }
            theme->setCurrentIndex(std::max(0, theme->findData(manager->lookTheme())));
        };
        fillThemes();
        auto *themeRow = new QHBoxLayout;
        themeRow->setSpacing(8);
        themeRow->addWidget(theme);
        auto *create = new QPushButton(QDialog::tr("Create a theme…"));
        create->setAutoDefault(false);
        themeRow->addWidget(create);
        themeRow->addStretch();
        form->addWidget(new QLabel(QDialog::tr("Theme:")), row, 0);
        form->addLayout(themeRow, row++, 1);

        auto *swatches = new QHBoxLayout;
        swatches->setSpacing(8);
        auto *buttons = new QButtonGroup(group);
        buttons->setExclusive(true);
        auto *hex = new QLineEdit;
        hex->setPlaceholderText(QStringLiteral("#RRGGBB"));
        hex->setMaxLength(7);
        hex->setFixedWidth(96);
        hex->setValidator(new QRegularExpressionValidator(QRegularExpression(QStringLiteral("#[0-9A-Fa-f]{0,6}")), hex));
        auto *ratio = new QLabel;
        ratio->setEnabled(false);
        for (const DoseThemeSource::Choice &c : DoseThemeSource::accentPresets())
        {
            auto *b = new QToolButton;
            b->setCheckable(true);
            b->setToolTip(c.name);
            b->setFixedSize(24, 24);
            b->setProperty("accent", c.id);
            buttons->addButton(b);
            swatches->addWidget(b);
        }
        swatches->addSpacing(8);
        swatches->addWidget(hex);
        swatches->addStretch();
        form->addWidget(new QLabel(QDialog::tr("Accent colour:")), row, 0);
        form->addLayout(swatches, row++, 1);
        form->addWidget(ratio, row++, 1);

        auto *density = new QComboBox;
        for (const DoseThemeSource::Choice &c : DoseThemeSource::densities())
            density->addItem(c.name, c.id);
        density->setCurrentIndex(std::max(0, density->findData(manager->lookDensity())));
        form->addWidget(new QLabel(QDialog::tr("Density:")), row, 0);
        form->addWidget(density, row++, 1, Qt::AlignLeft);

        auto *icons = new QComboBox;
        for (const auto &[px, name] : {std::pair {16, QDialog::tr("Small")}, std::pair {20, QDialog::tr("Medium")}, std::pair {24, QDialog::tr("Large")}})
            icons->addItem(name, px);
        icons->setCurrentIndex(std::max(0, icons->findData(DoseToolbar::iconSize())));
        form->addWidget(new QLabel(QDialog::tr("Toolbar icons:")), row, 0);
        form->addWidget(icons, row++, 1, Qt::AlignLeft);
        QObject::connect(icons, &QComboBox::activated, group, [icons](int)
        {
            QToolBar *bar = nullptr;
            for (QWidget *top : QApplication::topLevelWidgets())
            {
                if (auto *main = qobject_cast<QMainWindow *>(top))
                    bar = main->findChild<QToolBar *>(u"toolBar"_s);
            }
            DoseToolbar::setIconSize(bar, icons->currentData().toInt());
        });

        auto *bars = new QComboBox;
        bars->addItem(QDialog::tr("Line"), u"line"_s);
        bars->addItem(QDialog::tr("Bar"), u"bar"_s);
        bars->addItem(QDialog::tr("Filled, percentage inside"), u"filled"_s);
        bars->setCurrentIndex(std::max(0, bars->findData(DosePaint::progressStyle())));
        form->addWidget(new QLabel(QDialog::tr("Progress bars:")), row, 0);
        form->addWidget(bars, row++, 1, Qt::AlignLeft);
        QObject::connect(bars, &QComboBox::activated, group, [bars](int)
        {
            DosePaint::setProgressStyle(bars->currentData().toString());
        });

        auto *height = new QComboBox;
        height->addItem(QDialog::tr("Slim"), true);
        height->addItem(QDialog::tr("Regular"), false);
        height->setCurrentIndex(DoseToolbar::isSlim() ? 0 : 1);
        form->addWidget(new QLabel(QDialog::tr("Toolbar height:")), row, 0);
        form->addWidget(height, row++, 1, Qt::AlignLeft);
        QObject::connect(height, &QComboBox::activated, group, [height](int)
        {
            for (QWidget *top : QApplication::topLevelWidgets())
            {
                if (auto *main = qobject_cast<QMainWindow *>(top))
                    DoseToolbar::setSlim(main->findChild<QToolBar *>(u"toolBar"_s), height->currentData().toBool());
            }
        });

        // Shows the current choice: swatches tinted for the resolved theme,
        // the hex field with the accent's measured contrast.
        const auto refresh = [=]
        {
            const QString themeId = manager->resolvedTheme().isEmpty() ? QStringLiteral("light") : manager->resolvedTheme();
            const QString accent = manager->lookAccent();
            for (QAbstractButton *b : buttons->buttons())
            {
                const QString id = b->property("accent").toString();
                const QSignalBlocker blocker {b};
                b->setChecked(id == accent);
                b->setStyleSheet(QStringLiteral(
                    "QToolButton { background: %1; border: 2px solid palette(base); border-radius: 12px; }"
                    "QToolButton:checked { border: 2px solid palette(text); }")
                    .arg(DoseThemeSource::accentBase(id, themeId).name()));
            }
            const QColor base = DoseThemeSource::accentBase(accent, themeId);
            if (!hex->hasFocus())
                hex->setText(base.name().toUpper());
            const double r = DoseThemeSource::accentContrast(accent, themeId);
            ratio->setText((r < 3.0)
                ? QDialog::tr("%1:1, adjusted to 3:1 for this theme").arg(r, 0, 'f', 1)
                : QDialog::tr("%1:1 contrast").arg(r, 0, 'f', 1));
        };
        const auto apply = [=](const QString &accent)
        {
            manager->setLook(theme->currentData().toString(), accent, density->currentData().toString());
            refresh();
        };
        QObject::connect(buttons, &QButtonGroup::buttonClicked, group, [=](QAbstractButton *b)
        {
            apply(b->property("accent").toString());
        });
        QObject::connect(hex, &QLineEdit::editingFinished, group, [=]
        {
            const QColor c {hex->text()};
            if (c.isValid() && (hex->text().size() == 7))
                apply(c.name());
        });
        QObject::connect(theme, &QComboBox::activated, group, [=](int) { apply(manager->lookAccent()); });
        QObject::connect(create, &QPushButton::clicked, group, [=]
        {
            DoseThemeEditor editor {manager->resolvedTheme(), group->window()};
            if (editor.exec() != QDialog::Accepted)
                return;
            manager->setLook(editor.savedThemeId(), manager->lookAccent(), density->currentData().toString());
            fillThemes();
            refresh();
        });
        QObject::connect(density, &QComboBox::activated, group, [=](int) { apply(manager->lookAccent()); });
        refresh();

        if (!dose)
        {
            auto *note = new QLabel(QDialog::tr("These apply to tTorrent's built-in look. A custom UI theme is in use."));
            note->setWordWrap(true);
            form->addWidget(note, row++, 1);
        }

        auto *scale = new QComboBox;
        const double now = SettingsStorage::instance()->loadValue<double>(DoseThemeSource::scaleKey(), 1.0);
        for (const double s : {0.8, 0.9, 1.0, 1.1, 1.25, 1.5})
        {
            scale->addItem(QStringLiteral("%1 %").arg(qRound(s * 100)), s);
            if (qFuzzyCompare(s, now))
                scale->setCurrentIndex(scale->count() - 1);
        }
        auto *scaleRow = new QHBoxLayout;
        scaleRow->addWidget(scale);
        auto *restart = new QLabel(QDialog::tr("Takes effect after restarting tTorrent"));
        restart->setEnabled(false);
        scaleRow->addWidget(restart);
        scaleRow->addStretch();
        form->addWidget(new QLabel(QDialog::tr("Interface size:")), row, 0);
        form->addLayout(scaleRow, row++, 1);
        QObject::connect(scale, &QComboBox::activated, group, [scale](int)
        {
            SettingsStorage::instance()->storeValue(DoseThemeSource::scaleKey(), scale->currentData().toDouble());
        });
        form->setColumnStretch(1, 1);

        pageLayout->insertWidget(pageLayout->indexOf(first), group);

        // Moving from qBittorrent: one button, explained in a line.
        auto *move = new QGroupBox(QDialog::tr("Move from qBittorrent"));
        auto *moveRow = new QHBoxLayout(move);
        moveRow->setSpacing(12);
        const int count = DoseImport::qbittorrentTorrentCount();
        auto *moveText = new QLabel((count >= 0)
            ? QDialog::tr("Bring over %1 torrents with their progress, plus categories, tags, RSS and settings.").arg(count)
            : QDialog::tr("No qBittorrent profile found on this computer."));
        moveText->setWordWrap(true);
        auto *moveButton = new QPushButton(QDialog::tr("Import from qBittorrent…"));
        moveButton->setAutoDefault(false);
        moveButton->setEnabled(count >= 0);
        QObject::connect(moveButton, &QPushButton::clicked, move, [move] { DoseImport::askAndRestart(move->window()); });
        moveRow->addWidget(moveText, 1);
        moveRow->addWidget(moveButton);
        pageLayout->insertWidget(pageLayout->indexOf(first), move);
    }

    // Downloads › "When something happens": disk guard, unpacking, cards,
    // webhooks (DoseAutomation), after the page's own groups.
    void addGroupToPage(QWidget *page, QWidget *group, void (*save)(QWidget *), const bool atTop)
    {
        QGroupBox *first = page ? page->findChild<QGroupBox *>() : nullptr;
        auto *pageLayout = first ? qobject_cast<QBoxLayout *>(first->parentWidget()->layout()) : nullptr;
        if (!pageLayout)
        {
            delete group;
            return;
        }
        int at = atTop ? pageLayout->indexOf(first) : 0;
        for (int i = 0; !atTop && (i < pageLayout->count()); ++i)
        {
            if (qobject_cast<QGroupBox *>(pageLayout->itemAt(i)->widget()))
                at = i + 1;
        }
        pageLayout->insertWidget(at, group);
        if (auto *box = page->window()->findChild<QDialogButtonBox *>())
        {
            QObject::connect(box, &QDialogButtonBox::clicked, group, [box, group, save](QAbstractButton *button)
            {
                const QDialogButtonBox::ButtonRole role = box->buttonRole(button);
                if ((role == QDialogButtonBox::AcceptRole) || (role == QDialogButtonBox::ApplyRole))
                    save(group);
            });
        }
    }

    void addAutomationGroup(QWidget *downloadsPage)
    {
        addGroupToPage(downloadsPage, DoseAutomation::settingsGroup(), &DoseAutomation::saveSettings, false);
    }

    // Moves a widget into a new layout (Qt takes it out of its old one).
    void move(QWidget *w, QBoxLayout *to, const int index = -1)
    {
        if (w)
            to->insertWidget(index, w);
    }
}

void DoseDialogs::decorateOptions(QDialog *dialog, QSplitter *splitter, QListWidget *sections, QStackedWidget *pages)
{
    if (!dialog || !splitter || !sections || !pages)
        return;
    dialog->setProperty("doseOwnHeading", true);
    if ((splitter->indexOf(sections) != 0) || (splitter->indexOf(pages) != 1))
        return;

    // The title bar sits right above: no extra top margin in the dialog.
    if (QLayout *outer = dialog->layout())
    {
        const QMargins m = outer->contentsMargins();
        outer->setContentsMargins(m.left(), 0, m.right(), m.bottom());
    }

    // Sidebar: heading, search, sections.
    auto *sidebar = new QWidget;
    sidebar->setObjectName(QStringLiteral("doseSidebar"));
    auto *side = new QVBoxLayout(sidebar);
    side->setContentsMargins(12, 4, 12, 12);
    side->setSpacing(8);
    auto *heading = new QLabel(QDialog::tr("Settings"));
    heading->setObjectName(QStringLiteral("doseSettingsTitle"));
    auto *search = new QLineEdit;
    search->setObjectName(QStringLiteral("doseSearch"));
    search->setPlaceholderText(QDialog::tr("Search settings"));
    search->setClearButtonEnabled(true);
    splitter->replaceWidget(0, sidebar);
    side->addWidget(heading);
    side->addWidget(search);
    side->addWidget(sections, 1);

    // Sections as a list (16 px icon beside the name), not icon-over-label
    // tiles that cut names short.
    sections->setViewMode(QListView::ListMode);
    sections->setFlow(QListView::TopToBottom);
    sections->setWrapping(false);
    sections->setIconSize({15, 15});
    sections->setSpacing(1);
    sections->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    sections->setMinimumWidth(196);
    for (int i = 0; i < sections->count(); ++i)
    {
        QListWidgetItem *item = sections->item(i);
        item->setTextAlignment(Qt::AlignLeft | Qt::AlignVCenter);
        item->setSizeHint({0, 30});
        if (UIThemeManager::instance()->usesDoseTheme())
        {
            if (const QIcon icon = sectionIcon(pages->widget(i) ? pages->widget(i)->objectName() : QString()); !icon.isNull())
                item->setIcon(icon);
        }
    }
    sidebar->setMinimumWidth(220);

    // Pages scroll up and down only; the look is chosen in Look, so the
    // platform style picker would only fight it.
    for (QScrollArea *area : pages->findChildren<QScrollArea *>())
        area->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    DoseTrim::applyToOptions(dialog);
    if (UIThemeManager::instance()->usesDoseTheme())
    {
        for (const char *name : {"labelStyle", "comboStyle", "labelStyleHint"})
        {
            if (auto *w = dialog->findChild<QWidget *>(QLatin1String(name)))
                w->hide();
        }
    }

    // Content: the current section's name over its page.
    auto *content = new QWidget;
    auto *main = new QVBoxLayout(content);
    main->setContentsMargins(0, 0, 0, 0);
    main->setSpacing(0);
    auto *title = new QLabel;
    title->setObjectName(QStringLiteral("dosePageTitle"));
    auto *noResults = new QLabel(QDialog::tr("Nothing matches. Try another word, like port or ratio."));
    noResults->setObjectName(QStringLiteral("doseNoResults"));
    noResults->hide();
    splitter->replaceWidget(1, content);
    main->addWidget(title);
    main->addWidget(noResults);
    main->addWidget(pages, 1);

    const auto showTitle = [sections, title]
    {
        const QListWidgetItem *item = sections->currentItem();
        title->setText(item ? item->text() : QString());
    };
    QObject::connect(sections, &QListWidget::currentRowChanged, title, showTitle);
    showTitle();

    // Search, indexed on first use so opening the dialog costs nothing.
    auto index = std::make_shared<QHash<int, QString>>();
    QObject::connect(search, &QLineEdit::textChanged, sections, [=](const QString &text)
    {
        const QString q = text.trimmed().toLower();
        if (index->isEmpty())
        {
            for (int i = 0; i < pages->count(); ++i)
                index->insert(i, searchableText(pages->widget(i)));
        }
        int first = -1;
        for (int i = 0; i < sections->count(); ++i)
        {
            QListWidgetItem *item = sections->item(i);
            // Sections hidden by the build (no Web UI) stay hidden.
            if (item->data(Qt::UserRole + 77).toBool())
                continue;
            const bool match = q.isEmpty() || item->text().toLower().contains(q) || index->value(i).contains(q);
            item->setHidden(!match);
            if (match && (first < 0))
                first = i;
        }
        noResults->setVisible(first < 0);
        pages->setVisible(first >= 0);
        title->setVisible(first >= 0);
        const QListWidgetItem *current = sections->currentItem();
        if ((first >= 0) && (!current || current->isHidden()))
            sections->setCurrentRow(first);
    });
    for (int i = 0; i < sections->count(); ++i)
    {
        if (sections->item(i)->isHidden())
            sections->item(i)->setData(Qt::UserRole + 77, true);
    }

    addLookGroup(pages->widget(0));
    addAutomationGroup(pages->widget(1));
    addGroupToPage(pages->findChild<QWidget *>(QStringLiteral("tabWebUIPage")), DoseMcp::settingsGroup(), &DoseMcp::saveSettings, true);
    addGroupToPage(pages->widget(0), DoseUpdate::settingsGroup(), &DoseUpdate::saveSettings, false);
    // Long help texts (the run-program parameters) wrap instead of making
    // the page wider than the dialog.
    for (QLabel *label : pages->findChildren<QLabel *>())
    {
        if (!label->wordWrap() && (QTextDocumentFragment::fromHtml(label->text()).toPlainText().size() > 90))
            label->setWordWrap(true);
    }
    dialog->setStyleSheet(dialog->styleSheet() + QString::fromLatin1(OPTIONS_STYLE));
}

void DoseDialogs::decorateAddTorrent(QDialog *dialog)
{
    dialog->setProperty("doseOwnHeading", true);
    // Room either side of the splitter, so the columns' edges never touch.
    if (auto *split = dialog->findChild<QSplitter *>(QStringLiteral("splitter")))
    {
        split->setHandleWidth(14);
        split->setStretchFactor(0, 4);
        split->setStretchFactor(1, 5);
        QTimer::singleShot(0, split, [split]
        {
            const int total = split->width();
            if (total > 200)
                split->setSizes({total * 45 / 100, total * 55 / 100});
        });
        DosePaint::selectButtonsAsIcons(dialog);
        // The file name gets the room the size and priority columns leave.
        if (auto *tree = dialog->findChild<QTreeView *>(QStringLiteral("contentTreeView")))
        {
            QTimer::singleShot(0, tree, [tree]
            {
                const int rest = tree->header()->sectionSize(1) + tree->header()->sectionSize(3);
                tree->header()->resizeSection(0, std::max(160, tree->viewport()->width() - rest - 8));
            });
        }
        for (int i = 0; i < split->count(); ++i)
        {
            if (QLayout *l = split->widget(i)->layout())
            {
                QMargins m = l->contentsMargins();
                (i == 0) ? m.setRight(std::max(m.right(), 6)) : m.setLeft(std::max(m.left(), 6));
                l->setContentsMargins(m);
            }
        }
    }
    if (!dialog)
        return;
    auto *root = qobject_cast<QBoxLayout *>(dialog->layout());
    if (!root)
        return;

    // Header: "Add torrent", the torrent's name (the window title), size and date.
    auto *header = new QWidget;
    header->setObjectName(QStringLiteral("doseAddHeader"));
    auto *head = new QVBoxLayout(header);
    head->setContentsMargins(8, 4, 8, 12);
    head->setSpacing(4);
    auto *title = new QLabel(QDialog::tr("Add torrent"));
    title->setObjectName(QStringLiteral("doseAddTitle"));
    auto *name = new QLabel(dialog->windowTitle());
    name->setObjectName(QStringLiteral("doseAddName"));
    name->setTextInteractionFlags(Qt::TextSelectableByMouse);
    QObject::connect(dialog, &QWidget::windowTitleChanged, name, &QLabel::setText);
    head->addWidget(title);
    head->addWidget(name);
    auto *summary = new QHBoxLayout;
    summary->setSpacing(28);
    for (const auto &[key, value] : {std::pair {"labelSize", "labelSizeData"}, std::pair {"labelDate", "labelDateData"}})
    {
        auto *k = dialog->findChild<QLabel *>(QString::fromLatin1(key));
        auto *v = dialog->findChild<QLabel *>(QString::fromLatin1(value));
        if (!k || !v)
            continue;
        k->setProperty("doseRole", QStringLiteral("key"));
        auto *pair = new QVBoxLayout;
        pair->setSpacing(2);
        pair->addWidget(k);
        pair->addWidget(v);
        summary->addLayout(pair);
    }
    // Health: filled in by DoseHealth::check once the torrent is known.
    {
        auto *k = new QLabel(QDialog::tr("Health:"));
        k->setProperty("doseRole", QStringLiteral("key"));
        auto *v = new QLabel(QStringLiteral("—"));
        v->setObjectName(QStringLiteral("doseHealthData"));
        auto *pair = new QVBoxLayout;
        pair->setSpacing(2);
        pair->addWidget(k);
        pair->addWidget(v);
        summary->addLayout(pair);
    }
    summary->addStretch();
    head->addLayout(summary);
    root->insertWidget(0, header);

    // Torrent information (hashes, comment) folds under the files.
    auto *info = dialog->findChild<QGroupBox *>(QStringLiteral("infoGroup"));
    auto *filesColumn = dialog->findChild<QWidget *>(QStringLiteral("layoutWidget"));
    auto *filesLayout = filesColumn ? qobject_cast<QBoxLayout *>(filesColumn->layout()) : nullptr;
    if (info && filesLayout)
    {
        auto *toggle = new QToolButton;
        toggle->setObjectName(QStringLiteral("doseInfoToggle"));
        toggle->setText(QDialog::tr("Torrent information"));
        toggle->setToolButtonStyle(Qt::ToolButtonTextBesideIcon);
        const QColor ink = dialog->palette().color(QPalette::WindowText);
        toggle->setIcon(chevron(false, ink));
        toggle->setIconSize({14, 14});
        toggle->setCheckable(true);
        info->setTitle(QString());
        info->setFlat(true);
        info->setObjectName(QStringLiteral("doseInfoBox"));
        info->hide();
        QObject::connect(toggle, &QToolButton::toggled, info, [toggle, info, ink](const bool open)
        {
            toggle->setIcon(chevron(open, ink));
            info->setVisible(open);
        });
        filesLayout->addWidget(toggle);
        move(info, filesLayout);
    }

    // Footer: "Never show again" on the same row as the buttons.
    auto *neverShow = dialog->findChild<QCheckBox *>(QStringLiteral("checkBoxNeverShow"));
    auto *buttonsRow = dialog->findChild<QHBoxLayout *>(QStringLiteral("horizontalLayout"));
    if (neverShow && buttonsRow)
    {
        buttonsRow->insertWidget(0, neverShow);
        buttonsRow->insertStretch(1);
    }

    if (dialog->size().width() < 900)
        dialog->resize(960, 680);
    dialog->setStyleSheet(dialog->styleSheet() + QString::fromLatin1(ADD_STYLE));
}
