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

#include "doseshortcuts.h"

#include <algorithm>
#include <functional>

#include <QAction>
#include <QApplication>
#include <QDialog>
#include <QDialogButtonBox>
#include <QFrame>
#include <QGroupBox>
#include <QGridLayout>
#include <QKeyEvent>
#include <QKeySequenceEdit>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QMainWindow>
#include <QMenu>
#include <QMenuBar>
#include <QPainter>
#include <QPointer>
#include <QPushButton>
#include <QScrollArea>
#include <QStyledItemDelegate>
#include <QToolButton>
#include <QVBoxLayout>

#include "base/global.h"
#include "base/settingsstorage.h"
#include "dosepaint.h"
#include "gui/uithememanager.h"

namespace
{
    const QString KEYS_KEY = u"Appearance/Shortcuts"_s;
    const char DEFAULT_PROP[] = "doseDefaultShortcut";

    struct Command
    {
        QAction *action;
        QString section;
    };

    QString plain(QString text)
    {
        return text.remove(u'&').remove(u"..."_s).remove(QChar(0x2026)).trimmed();
    }

    // Every visible command in the menu bar, with the top menu it lives in.
    QList<Command> commands(QMainWindow *window)
    {
        QList<Command> list;
        // Never window->menuBar(): it would replace the custom title bar with a new bar.
        auto *bar = window->findChild<QMenuBar *>();
        if (!bar)
            return list;
        std::function<void (QMenu *, const QString &)> walk = [&](QMenu *menu, const QString &section)
        {
            for (QAction *a : menu->actions())
            {
                if (a->isSeparator() || !a->isVisible())
                    continue;
                if (QMenu *sub = a->menu())
                    walk(sub, section);
                else if (!a->objectName().isEmpty() && !plain(a->text()).isEmpty())
                    list.append({a, section});
            }
        };
        for (QAction *top : bar->actions())
        {
            if (top->menu() && top->isVisible())
                walk(top->menu(), plain(top->text()));
        }
        return list;
    }

    // Commands without an icon of their own get one from tTorrent's set,
    // so every row in the palette and the shortcuts sheet has one.
    QIcon iconFor(const QAction *action)
    {
        if (!action->icon().isNull())
            return action->icon();
        static const QHash<QString, QString> icons {
            {u"actionTopToolBar"_s, u"preferences-desktop"_s},
            {u"actionShowStatusbar"_s, u"speedometer"_s},
            {u"actionShowFiltersSidebar"_s, u"view-categories"_s},
            {u"actionSpeedInTitleBar"_s, u"chart-line"_s},
            {u"actionRSSReader"_s, u"application-rss"_s},
            {u"actionExecutionLogs"_s, u"folder-documents"_s},
            {u"actionNormalMessages"_s, u"help-about"_s},
            {u"actionInformationMessages"_s, u"help-about"_s},
            {u"actionWarningMessages"_s, u"dialog-warning"_s},
            {u"actionCriticalMessages"_s, u"error"_s},
            {u"actionAutoShutdownDisabled"_s, u"task-reject"_s},
            {u"actionAutoExit"_s, u"application-exit"_s},
            {u"actionAutoSuspend"_s, u"pause-session"_s},
            {u"actionAutoHibernate"_s, u"paused"_s},
            {u"actionAutoReboot"_s, u"view-refresh"_s},
            {u"actionAutoShutdown"_s, u"torrent-stop"_s},
            {u"actionToggleVisibility"_s, u"view-preview"_s},
            {u"actionMinimize"_s, u"go-down"_s},
            {u"actionUseAlternativeSpeedLimits"_s, u"slow_off"_s},
            {u"actionSetGlobalSpeedLimits"_s, u"speedometer"_s},
            {u"actionLock"_s, u"object-locked"_s},
            {u"actionStatistics"_s, u"view-statistics"_s},
            {u"actionOpenDestinationFolder"_s, u"directory"_s},
            {u"actionResumeSession"_s, u"torrent-start"_s},
            {u"actionPauseSession"_s, u"pause-session"_s},
            {u"actionTopQueuePos"_s, u"go-top"_s},
            {u"actionIncreaseQueuePos"_s, u"go-up"_s},
            {u"actionDecreaseQueuePos"_s, u"go-down"_s},
            {u"actionBottomQueuePos"_s, u"go-bottom"_s},
            {u"actionDocumentation"_s, u"help-contents"_s},
            {u"actionAbout"_s, u"help-about"_s},
            {u"actionCloseWindow"_s, u"application-exit"_s},
        };
        return UIThemeManager::instance()->getIcon(icons.value(action->objectName(), u"configure"_s));
    }

    QHash<QString, QString> savedKeys()
    {
        QHash<QString, QString> keys;
        for (const QString &pair : SettingsStorage::instance()->loadValue<QStringList>(KEYS_KEY))
        {
            const qsizetype eq = pair.indexOf(u'=');
            if (eq > 0)
                keys.insert(pair.left(eq), pair.mid(eq + 1));
        }
        return keys;
    }

    void saveKeys(const QHash<QString, QString> &keys)
    {
        QStringList list;
        for (auto it = keys.cbegin(); it != keys.cend(); ++it)
            list.append(it.key() + u'=' + it.value());
        list.sort();
        SettingsStorage::instance()->storeValue(KEYS_KEY, list);
    }

    QKeySequence defaultOf(const QAction *action)
    {
        return action->property(DEFAULT_PROP).value<QKeySequence>();
    }

    // Shortcuts that are not menu commands, shown read-only.
    QList<std::pair<QString, QKeySequence>> fixedKeys()
    {
        return {
            {QDialog::tr("Filter torrents"), QKeySequence(QKeySequence::Find)},
            {QDialog::tr("Transfers tab"), QKeySequence(Qt::ALT | Qt::Key_1)},
            {QDialog::tr("RSS tab"), QKeySequence(Qt::ALT | Qt::Key_3)},
            {QDialog::tr("Execution log tab"), QKeySequence(Qt::ALT | Qt::Key_4)},
            {QDialog::tr("Delete selected torrents"), QKeySequence(QKeySequence::Delete)},
        };
    }

    QLabel *keycap(const QKeySequence &keys)
    {
        auto *cap = new QLabel(keys.toString(QKeySequence::NativeText));
        cap->setProperty("doseKeycap", true);
        return cap;
    }

    // The palette's rows: the command on the left, its keys on the right.
    class PaletteDelegate final : public QStyledItemDelegate
    {
    public:
        using QStyledItemDelegate::QStyledItemDelegate;

        void paint(QPainter *painter, const QStyleOptionViewItem &option, const QModelIndex &index) const override
        {
            QStyleOptionViewItem opt = option;
            initStyleOption(&opt, index);
            const QString keys = index.data(Qt::UserRole + 1).toString();
            const QString section = index.data(Qt::UserRole + 2).toString();
            opt.text.clear();
            const QWidget *widget = option.widget;
            widget->style()->drawControl(QStyle::CE_ItemViewItem, &opt, painter, widget);

            const QRect area = option.rect.adjusted(40, 0, -12, 0);
            painter->save();
            QFont font = option.font;
            painter->setFont(font);
            painter->setPen(DosePaint::token(u"text"_s));
            const int keysWidth = keys.isEmpty() ? 0 : (QFontMetrics(DosePaint::numberFont(font)).horizontalAdvance(keys) + 16);
            const int sectionWidth = QFontMetrics(font).horizontalAdvance(section) + 12;
            const QRect textRect = area.adjusted(0, 0, -(keysWidth + sectionWidth + 8), 0);
            painter->drawText(textRect, Qt::AlignVCenter | Qt::AlignLeft
                , QFontMetrics(font).elidedText(index.data(Qt::DisplayRole).toString(), Qt::ElideRight, textRect.width()));
            painter->setPen(DosePaint::token(u"faint"_s));
            painter->drawText(QRect(textRect.right() + 8, area.top(), sectionWidth, area.height()), Qt::AlignVCenter | Qt::AlignLeft, section);
            if (!keys.isEmpty())
            {
                const QRect cap {area.right() - keysWidth + 1, area.center().y() - 11, keysWidth, 22};
                painter->setRenderHint(QPainter::Antialiasing);
                painter->setPen(DosePaint::token(u"line"_s));
                painter->setBrush(DosePaint::token(u"well"_s));
                painter->drawRoundedRect(QRectF(cap).adjusted(0.5, 0.5, -0.5, -0.5), 6, 6);
                painter->setFont(DosePaint::numberFont(font));
                painter->setPen(DosePaint::token(u"muted"_s));
                painter->drawText(cap, Qt::AlignCenter, keys);
            }
            painter->restore();
        }

        QSize sizeHint(const QStyleOptionViewItem &option, const QModelIndex &index) const override
        {
            return {QStyledItemDelegate::sizeHint(option, index).width(), 36};
        }
    };

    class Palette final : public QFrame
    {
    public:
        explicit Palette(QMainWindow *window)
            : QFrame(window, Qt::Popup | Qt::FramelessWindowHint | Qt::NoDropShadowWindowHint)
            , m_window {window}
        {
            setObjectName(u"dosePalette"_s);
            setAttribute(Qt::WA_DeleteOnClose);
            setAttribute(Qt::WA_TranslucentBackground);
            auto *outer = new QVBoxLayout(this);
            outer->setContentsMargins(0, 0, 0, 0);
            auto *card = new QFrame;
            card->setObjectName(u"dosePaletteCard"_s);
            outer->addWidget(card);
            auto *column = new QVBoxLayout(card);
            column->setContentsMargins(8, 8, 8, 8);
            column->setSpacing(6);

            m_edit = new QLineEdit;
            m_edit->setObjectName(u"dosePaletteEdit"_s);
            m_edit->setPlaceholderText(tr("Type a command"));
            m_edit->addAction(UIThemeManager::instance()->getIcon(u"edit-find"_s), QLineEdit::LeadingPosition);
            m_edit->installEventFilter(this);
            column->addWidget(m_edit);

            m_list = new QListWidget;
            m_list->setObjectName(u"dosePaletteList"_s);
            m_list->setItemDelegate(new PaletteDelegate(m_list));
            m_list->setIconSize({16, 16});
            m_list->setFrameShape(QFrame::NoFrame);
            m_list->setUniformItemSizes(true);
            m_list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
            column->addWidget(m_list);

            m_empty = new QLabel(tr("No command matches."));
            m_empty->setObjectName(u"dosePaletteEmpty"_s);
            m_empty->setAlignment(Qt::AlignCenter);
            m_empty->hide();
            column->addWidget(m_empty);

            auto *hint = new QLabel(tr("Enter to run  ·  Esc to close  ·  %1 for all shortcuts")
                .arg(QKeySequence(Qt::CTRL | Qt::Key_Slash).toString(QKeySequence::NativeText)));
            hint->setObjectName(u"dosePaletteHint"_s);
            column->addWidget(hint);

            for (const Command &c : commands(window))
            {
                auto *item = new QListWidgetItem(iconFor(c.action), plain(c.action->text()), m_list);
                item->setData(Qt::UserRole, QVariant::fromValue(static_cast<QObject *>(c.action)));
                item->setData(Qt::UserRole + 1, c.action->shortcut().toString(QKeySequence::NativeText));
                item->setData(Qt::UserRole + 2, c.section);
            }
            connect(m_edit, &QLineEdit::textChanged, this, &Palette::filter);
            connect(m_list, &QListWidget::itemActivated, this, &Palette::run);
            connect(m_list, &QListWidget::itemClicked, this, &Palette::run);
            filter();

            const int width = std::min(620, window->width() - 80);
            resize(width, 420);
            const QPoint origin = window->mapToGlobal(QPoint((window->width() - width) / 2, 64));
            move(origin);
        }

        void open()
        {
            show();
            m_edit->setFocus();
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if ((watched == m_edit) && (event->type() == QEvent::KeyPress))
            {
                const auto *key = static_cast<QKeyEvent *>(event);
                if ((key->key() == Qt::Key_Down) || (key->key() == Qt::Key_Up))
                {
                    step((key->key() == Qt::Key_Down) ? 1 : -1);
                    return true;
                }
                if ((key->key() == Qt::Key_Return) || (key->key() == Qt::Key_Enter))
                {
                    run(m_list->currentItem());
                    return true;
                }
            }
            return QFrame::eventFilter(watched, event);
        }

    private:
        // Every word typed must appear in the command or its menu.
        void filter()
        {
            const QStringList words = m_edit->text().split(u' ', Qt::SkipEmptyParts);
            QListWidgetItem *first = nullptr;
            for (int i = 0; i < m_list->count(); ++i)
            {
                QListWidgetItem *item = m_list->item(i);
                const QString haystack = item->text() + u' ' + item->data(Qt::UserRole + 2).toString();
                const bool match = std::all_of(words.cbegin(), words.cend(), [&haystack](const QString &w)
                {
                    return haystack.contains(w, Qt::CaseInsensitive);
                });
                auto *action = qobject_cast<QAction *>(item->data(Qt::UserRole).value<QObject *>());
                item->setHidden(!match || !action || !action->isEnabled());
                if (!item->isHidden() && !first)
                    first = item;
            }
            m_list->setCurrentItem(first);
            m_list->setVisible(first != nullptr);
            m_empty->setVisible(first == nullptr);
        }

        void step(const int delta)
        {
            int row = m_list->currentRow();
            for (int i = 0; i < m_list->count(); ++i)
            {
                row = (row + delta + m_list->count()) % m_list->count();
                if (!m_list->item(row)->isHidden())
                {
                    m_list->setCurrentRow(row);
                    return;
                }
            }
        }

        void run(QListWidgetItem *item)
        {
            if (!item)
                return;
            QPointer<QAction> action = qobject_cast<QAction *>(item->data(Qt::UserRole).value<QObject *>());
            close();
            if (action && action->isEnabled())
                QMetaObject::invokeMethod(action, &QAction::trigger, Qt::QueuedConnection);
        }

        QMainWindow *m_window = nullptr;
        QLineEdit *m_edit = nullptr;
        QListWidget *m_list = nullptr;
        QLabel *m_empty = nullptr;
    };

    // The command list under the title bar's search field. It never takes
    // focus: typing stays in the field, which filters the torrent list.
    class SearchDrop final : public QFrame
    {
    public:
        SearchDrop(QMainWindow *window, QLineEdit *edit)
            : QFrame(window, Qt::Tool | Qt::FramelessWindowHint | Qt::NoDropShadowWindowHint | Qt::WindowDoesNotAcceptFocus)
            , m_window {window}
            , m_edit {edit}
        {
            setObjectName(u"doseSearchDrop"_s);
            setAttribute(Qt::WA_TranslucentBackground);
            setAttribute(Qt::WA_ShowWithoutActivating);
            auto *outer = new QVBoxLayout(this);
            outer->setContentsMargins(0, 0, 0, 0);
            auto *card = new QFrame;
            card->setObjectName(u"dosePaletteCard"_s);
            outer->addWidget(card);
            auto *column = new QVBoxLayout(card);
            column->setContentsMargins(8, 8, 8, 8);
            column->setSpacing(6);

            m_title = new QLabel;
            m_title->setObjectName(u"dosePaletteHint"_s);
            column->addWidget(m_title);

            m_list = new QListWidget;
            m_list->setObjectName(u"dosePaletteList"_s);
            m_list->setItemDelegate(new PaletteDelegate(m_list));
            m_list->setIconSize({16, 16});
            m_list->setFrameShape(QFrame::NoFrame);
            m_list->setUniformItemSizes(true);
            m_list->setFocusPolicy(Qt::NoFocus);
            m_list->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
            column->addWidget(m_list);

            auto *hint = new QLabel(QDialog::tr("Typing filters the torrent list  ·  Enter runs a command  ·  Esc closes"));
            hint->setObjectName(u"dosePaletteHint"_s);
            column->addWidget(hint);

            connect(m_list, &QListWidget::itemClicked, this, &SearchDrop::run);
            connect(edit, &QLineEdit::textChanged, this, [this] { refresh(false); });
            edit->installEventFilter(this);
            window->installEventFilter(this);
        }

        // Ctrl+K: focus the field and list every command.
        void openAll()
        {
            m_edit->setFocus();
            m_edit->selectAll();
            refresh(true);
        }

    protected:
        bool eventFilter(QObject *watched, QEvent *event) override
        {
            if (watched == m_window)
            {
                switch (event->type())
                {
                case QEvent::Move:
                case QEvent::Resize:
                case QEvent::WindowStateChange:
                case QEvent::Hide:
                case QEvent::WindowDeactivate:
                    hide();
                    break;
                default:
                    break;
                }
                return QFrame::eventFilter(watched, event);
            }
            if (event->type() == QEvent::FocusOut)
            {
                hide();
            }
            else if ((event->type() == QEvent::KeyPress) && isVisible())
            {
                const auto *key = static_cast<QKeyEvent *>(event);
                switch (key->key())
                {
                case Qt::Key_Down:
                case Qt::Key_Up:
                    step((key->key() == Qt::Key_Down) ? 1 : -1);
                    return true;
                case Qt::Key_Return:
                case Qt::Key_Enter:
                    if (m_list->currentItem())
                    {
                        run(m_list->currentItem());
                        return true;
                    }
                    break;
                case Qt::Key_Escape:
                    hide();
                    return true;
                default:
                    break;
                }
            }
            return QFrame::eventFilter(watched, event);
        }

    private:
        void refresh(const bool all)
        {
            const QString text = m_edit->text().trimmed();
            // Ctrl+K lists everything; typing only while the field has focus.
            if (!all && (!m_edit->hasFocus() || text.isEmpty()))
            {
                hide();
                return;
            }
            const QStringList words = all ? QStringList() : text.split(u' ', Qt::SkipEmptyParts);
            m_list->clear();
            for (const Command &c : commands(m_window))
            {
                if (!c.action->isEnabled())
                    continue;
                const QString name = plain(c.action->text());
                const QString haystack = name + u' ' + c.section;
                const bool match = std::all_of(words.cbegin(), words.cend(), [&haystack](const QString &w)
                {
                    return haystack.contains(w, Qt::CaseInsensitive);
                });
                if (!match)
                    continue;
                auto *item = new QListWidgetItem(iconFor(c.action), name, m_list);
                item->setData(Qt::UserRole, QVariant::fromValue(static_cast<QObject *>(c.action)));
                item->setData(Qt::UserRole + 1, c.action->shortcut().toString(QKeySequence::NativeText));
                item->setData(Qt::UserRole + 2, c.section);
            }
            if (m_list->count() == 0)
            {
                hide();
                return;
            }
            m_title->setText(all ? QDialog::tr("Commands") : QDialog::tr("Commands matching \"%1\"").arg(text));
            m_list->setCurrentRow(all ? -1 : 0);
            place();
            if (!isVisible())
                show();
        }

        void place()
        {
            const int rows = std::min(m_list->count(), 8);
            const int rowHeight = std::max(1, m_list->sizeHintForRow(0));
            m_list->setFixedHeight((rows * rowHeight) + 4);
            const int width = std::max(m_edit->width(), 460);
            adjustSize();
            resize(width, sizeHint().height());
            move(m_edit->mapToGlobal(QPoint((m_edit->width() - width) / 2, m_edit->height() + 6)));
        }

        void step(const int delta)
        {
            const int count = m_list->count();
            if (count == 0)
                return;
            const int row = m_list->currentRow();
            m_list->setCurrentRow((row < 0) ? ((delta > 0) ? 0 : (count - 1)) : ((row + delta + count) % count));
        }

        void run(QListWidgetItem *item)
        {
            if (!item)
                return;
            QPointer<QAction> action = qobject_cast<QAction *>(item->data(Qt::UserRole).value<QObject *>());
            hide();
            // The text was a command name, not a torrent filter.
            m_edit->clear();
            if (action && action->isEnabled())
                QMetaObject::invokeMethod(action, &QAction::trigger, Qt::QueuedConnection);
        }

        QMainWindow *m_window = nullptr;
        QLineEdit *m_edit = nullptr;
        QLabel *m_title = nullptr;
        QListWidget *m_list = nullptr;
    };

    class Sheet final : public QDialog
    {
    public:
        explicit Sheet(QMainWindow *window)
            : QDialog(window)
            , m_window {window}
        {
            setWindowTitle(tr("Keyboard shortcuts"));
            setAttribute(Qt::WA_DeleteOnClose);
            auto *column = new QVBoxLayout(this);
            column->setContentsMargins(20, 12, 20, 16);
            column->setSpacing(10);

            auto *head = new QHBoxLayout;
            auto *intro = new QLabel(tr("Click a key to change it, then press the new keys. Esc cancels, Backspace clears."));
            intro->setObjectName(u"doseMuted"_s);
            intro->setWordWrap(true);
            head->addWidget(intro, 1);
            m_search = new QLineEdit;
            m_search->setPlaceholderText(tr("Search shortcuts"));
            m_search->addAction(UIThemeManager::instance()->getIcon(u"edit-find"_s), QLineEdit::LeadingPosition);
            m_search->setFixedWidth(240);
            head->addWidget(m_search);
            column->addLayout(head);

            m_conflict = new QLabel;
            m_conflict->hide();
            column->addWidget(m_conflict);

            auto *scroll = new QScrollArea;
            scroll->setWidgetResizable(true);
            scroll->setFrameShape(QFrame::NoFrame);
            scroll->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
            auto *body = new QWidget;
            auto *grid = new QGridLayout(body);
            grid->setContentsMargins(0, 0, 4, 0);
            grid->setHorizontalSpacing(14);
            grid->setVerticalSpacing(6);
            scroll->setWidget(body);
            column->addWidget(scroll, 1);

            // One card per menu, two columns of cards.
            QList<QString> order;
            QHash<QString, QList<QAction *>> bySection;
            for (const Command &c : commands(window))
            {
                if (!bySection.contains(c.section))
                    order.append(c.section);
                bySection[c.section].append(c.action);
            }
            int index = 0;
            for (const QString &section : order)
                addCard(grid, index++, section, bySection.value(section));
            addFixedCard(grid, index++);
            grid->setRowStretch((index + 1) / 2, 1);

            auto *buttons = new QDialogButtonBox;
            QPushButton *reset = buttons->addButton(tr("Reset all"), QDialogButtonBox::ResetRole);
            QPushButton *done = buttons->addButton(QDialogButtonBox::Close);
            done->setDefault(true);
            reset->setAutoDefault(false);
            connect(reset, &QPushButton::clicked, this, &Sheet::resetAll);
            connect(done, &QPushButton::clicked, this, &QDialog::accept);
            column->addWidget(buttons);

            connect(m_search, &QLineEdit::textChanged, this, &Sheet::filter);
            resize(860, std::min(680, window->height()));
        }

    private:
        struct Row
        {
            QAction *action = nullptr;
            QWidget *label = nullptr;
            QWidget *keys = nullptr;
            QToolButton *revert = nullptr;
            QKeySequenceEdit *edit = nullptr;
            QGroupBox *card = nullptr;
            QString text;
        };

        void addCard(QGridLayout *grid, const int index, const QString &title, const QList<QAction *> &actions)
        {
            auto *card = new QGroupBox(title);
            auto *rows = new QGridLayout(card);
            rows->setHorizontalSpacing(8);
            rows->setVerticalSpacing(2);
            rows->setColumnStretch(1, 1);
            int r = 0;
            for (QAction *action : actions)
            {
                Row row;
                row.action = action;
                row.card = card;
                row.text = plain(action->text());
                auto *icon = new QLabel;
                icon->setPixmap(iconFor(action).pixmap(16, 16));
                icon->setFixedWidth(18);
                auto *name = new QLabel(row.text);
                row.label = name;
                row.edit = new QKeySequenceEdit(action->shortcut());
                row.edit->setProperty("doseKeycap", true);
                row.edit->setMaximumSequenceLength(1);
                row.edit->setFixedWidth(150);
                if (auto *line = row.edit->findChild<QLineEdit *>())
                    line->setPlaceholderText(tr("Not set"));
                row.edit->setToolTip(tr("Click, then press the new keys"));
                row.keys = row.edit;
                row.revert = new QToolButton;
                row.revert->setIcon(UIThemeManager::instance()->getIcon(u"edit-clear"_s, u"edit-undo"_s));
                row.revert->setToolTip(tr("Back to the default (%1)")
                    .arg(defaultOf(action).isEmpty() ? tr("none") : defaultOf(action).toString(QKeySequence::NativeText)));
                row.revert->setAutoRaise(true);
                row.revert->setFixedSize(24, 24);
                rows->addWidget(icon, r, 0);
                rows->addWidget(name, r, 1);
                rows->addWidget(row.edit, r, 2);
                rows->addWidget(row.revert, r, 3);
                ++r;
                const qsizetype at = m_rows.size();
                m_rows.append(row);
                connect(row.edit, &QKeySequenceEdit::editingFinished, this, [this, at] { assign(at); });
                connect(row.revert, &QToolButton::clicked, this, [this, at]
                {
                    m_rows[at].edit->setKeySequence(defaultOf(m_rows[at].action));
                    assign(at);
                });
                refreshRevert(m_rows[at]);
            }
            grid->addWidget(card, index / 2, index % 2, Qt::AlignTop);
        }

        void addFixedCard(QGridLayout *grid, const int index)
        {
            auto *card = new QGroupBox(tr("Window"));
            auto *rows = new QGridLayout(card);
            rows->setVerticalSpacing(2);
            rows->setColumnStretch(0, 1);
            int r = 0;
            QList<std::pair<QString, QKeySequence>> keys = fixedKeys();
            keys.prepend({tr("Command palette"), QKeySequence(Qt::CTRL | Qt::Key_K)});
            keys.prepend({tr("Keyboard shortcuts"), QKeySequence(Qt::CTRL | Qt::Key_Slash)});
            for (const auto &[text, sequence] : keys)
            {
                auto *name = new QLabel(text);
                QLabel *cap = keycap(sequence);
                rows->addWidget(name, r, 0);
                rows->addWidget(cap, r, 1, Qt::AlignRight);
                Row row;
                row.label = name;
                row.keys = cap;
                row.card = card;
                row.text = text;
                m_rows.append(row);
                ++r;
            }
            grid->addWidget(card, index / 2, index % 2, Qt::AlignTop);
        }

        void refreshRevert(const Row &row)
        {
            if (row.revert)
                row.revert->setVisible(row.action->shortcut() != defaultOf(row.action));
        }

        // Takes the new keys unless another command already has them.
        void assign(const qsizetype at)
        {
            Row &row = m_rows[at];
            const QKeySequence keys = row.edit->keySequence();
            if (!keys.isEmpty())
            {
                for (const Row &other : std::as_const(m_rows))
                {
                    const bool taken = other.action ? ((other.action != row.action) && (other.action->shortcut() == keys))
                        : (qobject_cast<QLabel *>(other.keys) && (qobject_cast<QLabel *>(other.keys)->text() == keys.toString(QKeySequence::NativeText)));
                    if (taken)
                    {
                        showConflict(tr("%1 is already used by \"%2\".").arg(keys.toString(QKeySequence::NativeText), other.text));
                        const QSignalBlocker blocker {row.edit};
                        row.edit->setKeySequence(row.action->shortcut());
                        return;
                    }
                }
            }
            m_conflict->hide();
            row.action->setShortcut(keys);
            QHash<QString, QString> saved = savedKeys();
            if (keys == defaultOf(row.action))
                saved.remove(row.action->objectName());
            else
                saved.insert(row.action->objectName(), keys.toString(QKeySequence::PortableText));
            saveKeys(saved);
            refreshRevert(row);
        }

        void showConflict(const QString &text)
        {
            m_conflict->setText(text);
            DosePaint::warningBanner(m_conflict);
            m_conflict->show();
        }

        void resetAll()
        {
            for (Row &row : m_rows)
            {
                if (!row.action)
                    continue;
                row.action->setShortcut(defaultOf(row.action));
                const QSignalBlocker blocker {row.edit};
                row.edit->setKeySequence(defaultOf(row.action));
                refreshRevert(row);
            }
            saveKeys({});
            m_conflict->hide();
        }

        void filter()
        {
            const QString text = m_search->text().trimmed();
            QHash<QGroupBox *, bool> anyShown;
            for (const Row &row : std::as_const(m_rows))
            {
                const QString keys = row.action ? row.action->shortcut().toString(QKeySequence::NativeText)
                    : qobject_cast<QLabel *>(row.keys)->text();
                const bool match = text.isEmpty() || row.text.contains(text, Qt::CaseInsensitive) || keys.contains(text, Qt::CaseInsensitive)
                    || row.card->title().contains(text, Qt::CaseInsensitive);
                row.label->setVisible(match);
                row.keys->setVisible(match);
                if (row.revert)
                    row.revert->setVisible(match && (row.action->shortcut() != defaultOf(row.action)));
                anyShown[row.card] = anyShown.value(row.card) || match;
            }
            // Icons follow their label's row.
            for (QGroupBox *card : anyShown.keys())
            {
                card->setVisible(anyShown.value(card));
                auto *rows = qobject_cast<QGridLayout *>(card->layout());
                for (int r = 0; rows && (r < rows->rowCount()); ++r)
                {
                    QLayoutItem *first = rows->itemAtPosition(r, 0);
                    QLayoutItem *second = rows->itemAtPosition(r, 1);
                    if (first && second && first->widget() && second->widget() && (rows->columnCount() > 2))
                        first->widget()->setVisible(second->widget()->isVisibleTo(card));
                }
            }
        }

        QMainWindow *m_window = nullptr;
        QLineEdit *m_search = nullptr;
        QLabel *m_conflict = nullptr;
        QList<Row> m_rows;
    };
}

void DoseShortcuts::install(QMainWindow *window)
{
    if (!window)
        return;
    const QHash<QString, QString> saved = savedKeys();
    for (const Command &c : commands(window))
    {
        c.action->setProperty(DEFAULT_PROP, c.action->shortcut());
        if (saved.contains(c.action->objectName()))
            c.action->setShortcut(QKeySequence(saved.value(c.action->objectName()), QKeySequence::PortableText));
    }

    QMenu *help = window->findChild<QMenu *>(u"menuHelp"_s);
    auto *sheet = new QAction(UIThemeManager::instance()->getIcon(u"preferences-desktop-keyboard"_s, u"help-contents"_s)
        , QDialog::tr("Keyboard shortcuts"), window);
    sheet->setObjectName(u"doseActionShortcuts"_s);
    sheet->setShortcut(Qt::CTRL | Qt::Key_Slash);
    QObject::connect(sheet, &QAction::triggered, window, [window] { showSheet(window); });
    auto *palette = new QAction(UIThemeManager::instance()->getIcon(u"edit-find"_s), QDialog::tr("Command palette"), window);
    palette->setObjectName(u"doseActionPalette"_s);
    palette->setShortcut(Qt::CTRL | Qt::Key_K);
    QObject::connect(palette, &QAction::triggered, window, [window] { showPalette(window); });
    window->addAction(sheet);
    window->addAction(palette);
    if (help)
    {
        QAction *first = help->actions().value(0);
        help->insertAction(first, palette);
        help->insertAction(first, sheet);
        help->insertSeparator(first);
    }
}

void DoseShortcuts::showSheet(QMainWindow *window)
{
    auto *sheet = new Sheet(window);
    sheet->open();
}

void DoseShortcuts::showPalette(QMainWindow *window)
{
    // The title bar's search field when there is one, else a floating palette.
    if (auto *drop = window->findChild<QFrame *>(u"doseSearchDrop"_s); drop && window->isVisible())
    {
        static_cast<SearchDrop *>(drop)->openAll();
        return;
    }
    auto *palette = new Palette(window);
    palette->open();
}

void DoseShortcuts::attachSearch(QMainWindow *window, QLineEdit *edit)
{
    if (!window || !edit || !DosePaint::active())
        return;
    new SearchDrop(window, edit);
}
