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

#include "doseviews.h"

#include <QAbstractItemView>
#include <QBoxLayout>
#include <QFrame>
#include <QHeaderView>
#include <QInputDialog>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLineEdit>
#include <QListWidget>
#include <QMenu>
#include <QRegularExpression>
#include <QTreeView>

#include "base/global.h"
#include "base/settingsstorage.h"
#include "gui/transferlistfilterswidgetitem.h"
#include "gui/uithememanager.h"

namespace
{
    const QString KEY_VIEWS = u"tTorrent/Views"_s;
    // The filter lists in the sidebar, by class, with the key a view saves them under.
    const QList<std::pair<const char *, QString>> FILTERS {
        {"StatusFilterWidget", u"status"_s},
        {"CategoryFilterWidget", u"category"_s},
        {"TagFilterWidget", u"tag"_s},
        {"TrackersFilterWidget", u"tracker"_s},
    };

    QString tr(const char *text)
    {
        return QCoreApplication::translate("DoseViews", text);
    }

    // "Downloading (3)" -> "Downloading": counts change, names don't.
    QString bare(const QString &text)
    {
        static const QRegularExpression count {u"\\s*\\(\\d+\\)$"_s};
        return QString(text).remove(count).trimmed();
    }

    QList<QJsonObject> loadViews()
    {
        QList<QJsonObject> views;
        for (const QString &json : SettingsStorage::instance()->loadValue<QStringList>(KEY_VIEWS))
        {
            const QJsonObject view = QJsonDocument::fromJson(json.toUtf8()).object();
            if (!view.value(u"name"_s).toString().isEmpty())
                views.append(view);
        }
        return views;
    }

    void saveViews(const QList<QJsonObject> &views)
    {
        QStringList list;
        for (const QJsonObject &view : views)
            list.append(QString::fromUtf8(QJsonDocument(view).toJson(QJsonDocument::Compact)));
        SettingsStorage::instance()->storeValue(KEY_VIEWS, list);
    }

    QAbstractItemView *filterView(QWidget *sidebar, const char *className)
    {
        for (QAbstractItemView *view : sidebar->findChildren<QAbstractItemView *>())
        {
            if (view->inherits(className))
                return view;
        }
        return nullptr;
    }

    QLineEdit *filterEdit(QWidget *anyWidget)
    {
        QWidget *window = anyWidget->window();
        if (auto *field = window->findChild<QFrame *>(u"doseSearchField"_s))
            return field->findChild<QLineEdit *>();
        return nullptr;
    }

    QJsonObject capture(QWidget *sidebar, QTreeView *list, const QString &name)
    {
        QJsonObject view {{u"name"_s, name}};
        for (const auto &[className, key] : FILTERS)
        {
            if (QAbstractItemView *v = filterView(sidebar, className); v && v->isVisible() && v->currentIndex().isValid())
                view.insert(key, bare(v->currentIndex().data(Qt::DisplayRole).toString()));
        }
        if (QLineEdit *edit = filterEdit(list))
            view.insert(u"text"_s, edit->text());
        view.insert(u"sortColumn"_s, list->header()->sortIndicatorSection());
        view.insert(u"sortOrder"_s, int(list->header()->sortIndicatorOrder()));
        return view;
    }

    void apply(QWidget *sidebar, QTreeView *list, const QJsonObject &view)
    {
        for (const auto &[className, key] : FILTERS)
        {
            QAbstractItemView *v = filterView(sidebar, className);
            if (!v || !v->model())
                continue;
            // A list the view didn't save goes back to its first row ("All").
            const QString wanted = view.value(key).toString();
            QModelIndex target = v->model()->index(0, 0);
            if (!wanted.isEmpty())
            {
                const QModelIndexList rows = v->model()->match(v->model()->index(0, 0), Qt::DisplayRole, wanted, -1
                    , Qt::MatchStartsWith | Qt::MatchRecursive);
                for (const QModelIndex &row : rows)
                {
                    if (bare(row.data(Qt::DisplayRole).toString()) == wanted)
                    {
                        target = row;
                        break;
                    }
                }
            }
            if (target.isValid())
                v->setCurrentIndex(target);
        }
        if (QLineEdit *edit = filterEdit(list))
            edit->setText(view.value(u"text"_s).toString());
        const int column = view.value(u"sortColumn"_s).toInt(-1);
        if (column >= 0)
            list->sortByColumn(column, Qt::SortOrder(view.value(u"sortOrder"_s).toInt()));
    }

    // The list itself: saved views, then "Save current view".
    class ViewsList final : public QListWidget
    {
    public:
        ViewsList(QWidget *sidebar, QTreeView *transfers)
            : m_sidebar {sidebar}
            , m_transfers {transfers}
        {
            setObjectName(u"doseViews"_s);
            setFrameShape(QFrame::NoFrame);
            setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
            setVerticalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
            setIconSize({16, 16});
            setContextMenuPolicy(Qt::CustomContextMenu);
            connect(this, &QListWidget::itemClicked, this, &ViewsList::activate);
            connect(this, &QWidget::customContextMenuRequested, this, &ViewsList::menu);
            refill();
        }

        QSize sizeHint() const override
        {
            int height = 2 * frameWidth();
            for (int i = 0; i < count(); ++i)
                height += sizeHintForRow(i);
            return {QListWidget::sizeHint().width(), height};
        }

    private:
        void refill()
        {
            clear();
            const QIcon viewIcon = UIThemeManager::instance()->getIcon(u"view-categories"_s);
            for (const QJsonObject &view : loadViews())
                new QListWidgetItem(viewIcon, view.value(u"name"_s).toString(), this);
            auto *add = new QListWidgetItem(UIThemeManager::instance()->getIcon(u"list-add"_s), tr("Save current view…"), this);
            add->setData(Qt::UserRole, true);
            add->setForeground(palette().color(QPalette::PlaceholderText));
            setFixedHeight(sizeHint().height());
        }

        void activate(QListWidgetItem *item)
        {
            if (!item)
                return;
            if (item->data(Qt::UserRole).toBool())
            {
                bool ok = false;
                const QString name = QInputDialog::getText(this, tr("Save view"), tr("Name:"), QLineEdit::Normal, {}, &ok).trimmed();
                if (ok && !name.isEmpty())
                {
                    QList<QJsonObject> views = loadViews();
                    views.removeIf([&name](const QJsonObject &v) { return v.value(u"name"_s).toString() == name; });
                    views.append(capture(m_sidebar, m_transfers, name));
                    saveViews(views);
                    refill();
                }
                clearSelection();
                return;
            }
            for (const QJsonObject &view : loadViews())
            {
                if (view.value(u"name"_s).toString() == item->text())
                    apply(m_sidebar, m_transfers, view);
            }
        }

        void menu(const QPoint &pos)
        {
            QListWidgetItem *item = itemAt(pos);
            if (!item || item->data(Qt::UserRole).toBool())
                return;
            const QString name = item->text();
            QMenu menu {this};
            menu.addAction(UIThemeManager::instance()->getIcon(u"view-refresh"_s), tr("Update to current view"), this, [this, name]
            {
                QList<QJsonObject> views = loadViews();
                for (QJsonObject &v : views)
                {
                    if (v.value(u"name"_s).toString() == name)
                        v = capture(m_sidebar, m_transfers, name);
                }
                saveViews(views);
            });
            menu.addAction(UIThemeManager::instance()->getIcon(u"edit-rename"_s), tr("Rename…"), this, [this, name]
            {
                bool ok = false;
                const QString renamed = QInputDialog::getText(this, tr("Rename view"), tr("Name:"), QLineEdit::Normal, name, &ok).trimmed();
                if (!ok || renamed.isEmpty())
                    return;
                QList<QJsonObject> views = loadViews();
                for (QJsonObject &v : views)
                {
                    if (v.value(u"name"_s).toString() == name)
                        v.insert(u"name"_s, renamed);
                }
                saveViews(views);
                refill();
            });
            menu.addAction(UIThemeManager::instance()->getIcon(u"list-remove"_s), tr("Remove"), this, [this, name]
            {
                QList<QJsonObject> views = loadViews();
                views.removeIf([&name](const QJsonObject &v) { return v.value(u"name"_s).toString() == name; });
                saveViews(views);
                refill();
            });
            menu.exec(viewport()->mapToGlobal(pos));
        }

        QWidget *m_sidebar = nullptr;
        QTreeView *m_transfers = nullptr;
    };
}

void DoseViews::install(QBoxLayout *sidebarLayout, QWidget *sidebar, QTreeView *transferList)
{
    if (!sidebarLayout || !sidebar || !transferList)
        return;
    auto *list = new ViewsList(sidebar, transferList);
    auto *item = new TransferListFiltersWidgetItem(tr("Views"), list, sidebar);
    const bool open = SettingsStorage::instance()->loadValue<bool>(u"tTorrent/ViewsOpen"_s, true);
    item->setChecked(open);
    list->setVisible(open);
    QObject::connect(item, &TransferListFiltersWidgetItem::toggled, list, [list](const bool checked)
    {
        list->setVisible(checked);
        SettingsStorage::instance()->storeValue(u"tTorrent/ViewsOpen"_s, checked);
    });
    sidebarLayout->insertWidget(0, item);
}
