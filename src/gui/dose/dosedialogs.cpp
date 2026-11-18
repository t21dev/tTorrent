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
#include <QToolButton>
#include <QTreeWidget>
#include <QTreeWidgetItemIterator>

#include <memory>
#include <utility>

#include "base/settingsstorage.h"
#include "gui/uithememanager.h"
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
    padding: 2px 6px 6px 6px;
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
    padding: 16px 24px 12px 24px;
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
    if ((splitter->indexOf(sections) != 0) || (splitter->indexOf(pages) != 1))
        return;

    // Sidebar: heading, search, sections.
    auto *sidebar = new QWidget;
    sidebar->setObjectName(QStringLiteral("doseSidebar"));
    auto *side = new QVBoxLayout(sidebar);
    side->setContentsMargins(12, 16, 12, 12);
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
    sections->setIconSize({16, 16});
    sections->setSpacing(1);
    sections->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
    sections->setMinimumWidth(196);
    for (int i = 0; i < sections->count(); ++i)
    {
        QListWidgetItem *item = sections->item(i);
        item->setTextAlignment(Qt::AlignLeft | Qt::AlignVCenter);
        item->setSizeHint({0, 34});
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
    dialog->setStyleSheet(dialog->styleSheet() + QString::fromLatin1(OPTIONS_STYLE));
}

void DoseDialogs::decorateAddTorrent(QDialog *dialog)
{
    // Room either side of the splitter, so the columns' edges never touch.
    if (auto *split = dialog->findChild<QSplitter *>(QStringLiteral("splitter")))
    {
        split->setHandleWidth(14);
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
