// SPDX-License-Identifier: MPL-2.0

//! COSMIC panel applet mode - mini clipboard manager popup.

use crate::config::{KeyBindingExt, KeybindingsConfig, load_config};
use crate::fl;
use crate::schema::entry::ClipboardEntry;
use crate::schema::{Codec, DBUS_NAME, DBUS_PATH, OxiCodeCodec};
use cosmic::Element;
use cosmic::app::{Application, Core, Task};
use cosmic::iced::event::{self, Event, listen_raw};
use cosmic::iced::platform_specific::shell::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{Length, Subscription, window};
use cosmic::iced_core::keyboard::{self, key::Named};
use cosmic::iced_core::widget::Id;
use cosmic::widget;
use tracing::debug;

const PREVIEW_LEN: usize = 24;
const LIST_HEIGHT: f32 = 350.0;

/// Keybinding config for applet subscription (must be static for non-capturing closure).
static APPLET_KEYBINDINGS: std::sync::RwLock<Option<KeybindingsConfig>> =
    std::sync::RwLock::new(None);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Page {
    All,
    Pinned,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(window::Id),
    OpenOverlay,
    ClearHistory,
    Settings,
    EntriesLoaded(Result<Vec<ClipboardEntry>, String>),
    CopyEntry(u64),
    DeleteEntry(u64),
    TogglePin(u64, bool),
    ActionDone(Result<bool, String>),
    // Search
    SearchChanged(String),
    SearchChar(char),
    SearchBackspace,
    // Page toggle
    SetPage(Page),
    // Selection
    SelectNext,
    SelectPrevious,
    SelectIndex(usize),
    ActivateSelected,
    // Keybinding actions on selected
    TogglePinSelected,
    DeleteSelected,
    DismissTips,
    DismissDaemonNotice,
}

pub struct AppletModel {
    core: Core,
    popup: Option<window::Id>,
    entries: Vec<ClipboardEntry>,
    error: Option<String>,
    page: Page,
    search_query: String,
    selected_index: Option<usize>,
    config: KeybindingsConfig,
    scrollable_id: Id,
    search_input_id: widget::Id,
    show_tips: bool,
    show_daemon_notice: bool,
}

impl AppletModel {
    fn filtered_entries(&self) -> Vec<&ClipboardEntry> {
        self.entries
            .iter()
            .filter(|e| match self.page {
                Page::All => true,
                Page::Pinned => e.pinned,
            })
            .filter(|e| {
                if self.search_query.is_empty() {
                    true
                } else {
                    e.as_text().is_some_and(|t| {
                        t.to_lowercase().contains(&self.search_query.to_lowercase())
                    })
                }
            })
            .collect()
    }

    fn selected_entry_id(&self) -> Option<u64> {
        let filtered = self.filtered_entries();
        self.selected_index
            .and_then(|idx| filtered.get(idx))
            .map(|e| e.id.0)
    }

    fn clamp_selection(&mut self) {
        let count = self.filtered_entries().len();
        if count == 0 {
            self.selected_index = None;
        } else if let Some(idx) = self.selected_index
            && idx >= count
        {
            self.selected_index = Some(count - 1);
        }
    }
}

impl Application for AppletModel {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = crate::schema::APPLET_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, cosmic::app::Task<Message>) {
        let config = load_config();
        if let Ok(mut kb) = APPLET_KEYBINDINGS.write() {
            *kb = Some(config.clone());
        }
        (
            Self {
                core,
                popup: None,
                entries: Vec::new(),
                error: None,
                page: Page::All,
                search_query: String::new(),
                selected_index: None,
                config,
                scrollable_id: Id::unique(),
                search_input_id: widget::Id::unique(),
                show_tips: crate::config::is_first_launch(),
                show_daemon_notice: false,
            },
            cosmic::app::Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        match message {
            Message::TogglePopup => {
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
                // Reset state on open
                self.search_query.clear();
                self.selected_index = None;
                self.page = Page::All;
                self.config = load_config();
                if let Ok(mut kb) = APPLET_KEYBINDINGS.write() {
                    *kb = Some(self.config.clone());
                }

                let id = window::Id::unique();
                self.popup = Some(id);
                let popup_settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().unwrap(),
                    id,
                    None,
                    None,
                    None,
                );
                return Task::batch(vec![
                    get_popup(popup_settings),
                    Task::perform(fetch_entries(), |r| {
                        cosmic::Action::App(Message::EntriesLoaded(r))
                    }),
                ]);
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
            Message::OpenOverlay => {
                crate::launcher::spawn(std::iter::empty::<&str>()).ok();
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
            Message::ClearHistory => {
                return Task::perform(call_daemon_action(DaemonAction::Clear), |r| {
                    cosmic::Action::App(Message::ActionDone(r))
                });
            }
            Message::Settings => {
                crate::launcher::spawn(["settings"]).ok();
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
            Message::EntriesLoaded(result) => match result {
                Ok(entries) => {
                    let was_empty = self.entries.is_empty();
                    self.entries = entries;
                    self.error = None;
                    self.show_daemon_notice = false;
                    if was_empty && !self.entries.is_empty() {
                        self.selected_index = Some(0);
                    }
                    self.clamp_selection();
                }
                Err(e) => {
                    self.show_daemon_notice = true;
                    self.error = Some(e);
                    self.entries.clear();
                    self.selected_index = None;
                }
            },
            Message::CopyEntry(id) => {
                if let Some(p) = self.popup.take() {
                    return Task::batch(vec![
                        destroy_popup(p),
                        Task::perform(call_daemon_action(DaemonAction::Paste(id)), |r| {
                            cosmic::Action::App(Message::ActionDone(r))
                        }),
                    ]);
                }
                return Task::perform(call_daemon_action(DaemonAction::Paste(id)), |r| {
                    cosmic::Action::App(Message::ActionDone(r))
                });
            }
            Message::DeleteEntry(id) => {
                return Task::perform(call_daemon_action(DaemonAction::Delete(id)), |r| {
                    cosmic::Action::App(Message::ActionDone(r))
                });
            }
            Message::TogglePin(id, pin) => {
                return Task::perform(
                    call_daemon_action(if pin {
                        DaemonAction::Pin(id)
                    } else {
                        DaemonAction::Unpin(id)
                    }),
                    |r| cosmic::Action::App(Message::ActionDone(r)),
                );
            }
            Message::ActionDone(result) => {
                match &result {
                    Err(e) if e.contains("pin_limit_reached") => {
                        self.error = Some(crate::fl!("pin-limit-reached"));
                    }
                    Err(e) => {
                        debug!(error = %e, "Action failed");
                    }
                    _ => {}
                }
                return Task::perform(fetch_entries(), |r| {
                    cosmic::Action::App(Message::EntriesLoaded(r))
                });
            }
            // Search
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.selected_index = if self.filtered_entries().is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
            Message::SearchChar(c) => {
                self.search_query.push(c);
                self.selected_index = if self.filtered_entries().is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
            Message::SearchBackspace => {
                self.search_query.pop();
                self.clamp_selection();
            }
            // Page toggle
            Message::SetPage(page) => {
                self.page = page;
                self.selected_index = if self.filtered_entries().is_empty() {
                    None
                } else {
                    Some(0)
                };
            }
            // Selection
            Message::SelectNext => {
                let count = self.filtered_entries().len();
                if count > 0 {
                    self.selected_index = Some(match self.selected_index {
                        Some(idx) if idx + 1 < count => idx + 1,
                        _ => 0,
                    });
                }
            }
            Message::SelectPrevious => {
                let count = self.filtered_entries().len();
                if count > 0 {
                    self.selected_index = Some(match self.selected_index {
                        Some(idx) if idx > 0 => idx - 1,
                        _ => count - 1,
                    });
                }
            }
            Message::SelectIndex(idx) => {
                let count = self.filtered_entries().len();
                if idx < count {
                    self.selected_index = Some(idx);
                }
            }
            Message::ActivateSelected => {
                if let Some(id) = self.selected_entry_id()
                    && let Some(p) = self.popup.take()
                {
                    return Task::batch(vec![
                        destroy_popup(p),
                        Task::perform(call_daemon_action(DaemonAction::Paste(id)), |r| {
                            cosmic::Action::App(Message::ActionDone(r))
                        }),
                    ]);
                }
            }
            // Keybinding actions on selected
            Message::TogglePinSelected => {
                if let Some(id) = self.selected_entry_id() {
                    let filtered = self.filtered_entries();
                    let pinned = self
                        .selected_index
                        .and_then(|idx| filtered.get(idx))
                        .is_some_and(|e| e.pinned);
                    return Task::perform(
                        call_daemon_action(if pinned {
                            DaemonAction::Unpin(id)
                        } else {
                            DaemonAction::Pin(id)
                        }),
                        |r| cosmic::Action::App(Message::ActionDone(r)),
                    );
                }
            }
            Message::DeleteSelected => {
                if let Some(id) = self.selected_entry_id() {
                    return Task::perform(call_daemon_action(DaemonAction::Delete(id)), |r| {
                        cosmic::Action::App(Message::ActionDone(r))
                    });
                }
            }
            Message::DismissTips => {
                self.show_tips = false;
                crate::config::mark_first_launch_done();
            }
            Message::DismissDaemonNotice => {
                self.show_daemon_notice = false;
            }
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let suggested_size = self.core.applet.suggested_size(true);
        self.core
            .applet
            .icon_button_from_handle(
                widget::icon::from_name(crate::schema::APP_ID)
                    .fallback(Some(widget::icon::IconFallback::Names(vec![
                        "folder-documents-symbolic".into(),
                    ])))
                    .size(suggested_size.0)
                    .into(),
            )
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: window::Id) -> Element<'_, Message> {
        let space_xs = cosmic::theme::spacing().space_xs;

        if self.show_tips {
            let tips = widget::column::with_capacity(7)
                .push(widget::text::title4(fl!("tips-title")))
                .push(widget::text::body(fl!("tips-shortcut")))
                .push(widget::text::body(fl!(
                    "tips-pin",
                    keybinding = self.config.toggle_pin.to_string()
                )))
                .push(widget::text::body(fl!(
                    "tips-delete",
                    keybinding = self.config.delete_entry.to_string()
                )))
                .push(widget::text::body(fl!(
                    "tips-tabs",
                    keybinding_all = self.config.tab_all.to_string(),
                    keybinding_pinned = self.config.tab_pinned.to_string()
                )))
                .push(
                    widget::row::with_capacity(2)
                        .push(widget::horizontal_space())
                        .push(
                            widget::button::text(fl!("tips-dismiss"))
                                .on_press(Message::DismissTips)
                                .class(cosmic::theme::Button::Suggested),
                        ),
                )
                .spacing(space_xs)
                .padding(8);

            return self.core.applet.popup_container(tips).into();
        }

        if self.show_daemon_notice {
            let notice = widget::column::with_capacity(6)
                .push(widget::text::title4(fl!("daemon-notice-title")))
                .push(widget::text::body(fl!("daemon-notice-body")))
                .push(
                    widget::container(
                        widget::text::body(fl!("daemon-notice-service"))
                            .font(cosmic::iced::Font::MONOSPACE),
                    )
                    .padding([4, 8])
                    .class(cosmic::theme::Container::Primary),
                )
                .push(widget::text::body(fl!("daemon-notice-or")))
                .push(
                    widget::container(
                        widget::text::body(fl!("daemon-notice-command"))
                            .font(cosmic::iced::Font::MONOSPACE),
                    )
                    .padding([4, 8])
                    .class(cosmic::theme::Container::Primary),
                )
                .push(
                    widget::row::with_capacity(2)
                        .push(widget::horizontal_space())
                        .push(
                            widget::button::text(fl!("daemon-notice-dismiss"))
                                .on_press(Message::DismissDaemonNotice)
                                .class(cosmic::theme::Button::Suggested),
                        ),
                )
                .spacing(space_xs)
                .padding(8);

            return self.core.applet.popup_container(notice).into();
        }

        let filtered = self.filtered_entries();

        // Search bar
        let search_bar = widget::text_input("Search…", &self.search_query)
            .on_input(Message::SearchChanged)
            .id(self.search_input_id.clone())
            .width(Length::Fill);

        // Page toggle row
        let all_btn = widget::button::text("All")
            .on_press(Message::SetPage(Page::All))
            .class(if self.page == Page::All {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            });

        let pinned_btn = widget::button::text("Pinned")
            .on_press(Message::SetPage(Page::Pinned))
            .class(if self.page == Page::Pinned {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            });

        // Entry list
        let list_content: Element<_> = if let Some(ref err) = self.error {
            widget::text::body(err.as_str()).into()
        } else if filtered.is_empty() {
            let msg = if self.entries.is_empty() {
                "Clipboard is empty"
            } else {
                "No matching entries"
            };
            widget::container(widget::text::body(msg))
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center)
                .into()
        } else {
            let mut list = widget::list_column().padding(0).spacing(4);

            for (idx, entry) in filtered.iter().enumerate() {
                let id = entry.id.0;
                let preview = entry.preview(PREVIEW_LEN);
                let pinned = entry.pinned;
                let is_selected = self.selected_index == Some(idx);

                let entry_text = widget::text(preview)
                    .width(Length::Fill)
                    .wrapping(cosmic::iced::widget::text::Wrapping::None)
                    .shaping(cosmic::iced::widget::text::Shaping::Basic);

                let buttons = widget::row::with_capacity(3)
                    .push(
                        widget::button::icon(widget::icon::from_name("edit-copy-symbolic"))
                            .extra_small()
                            .on_press(Message::CopyEntry(id))
                            .class(cosmic::theme::Button::Text),
                    )
                    .push(
                        widget::button::icon(if pinned {
                            widget::icon::from_name("starred-symbolic")
                        } else {
                            widget::icon::from_name("non-starred-symbolic")
                        })
                        .extra_small()
                        .on_press(Message::TogglePin(id, !pinned))
                        .class(if pinned {
                            cosmic::theme::Button::Suggested
                        } else {
                            cosmic::theme::Button::Text
                        }),
                    )
                    .push(
                        widget::button::icon(widget::icon::from_name("edit-delete-symbolic"))
                            .extra_small()
                            .on_press(Message::DeleteEntry(id))
                            .class(cosmic::theme::Button::Text),
                    )
                    .spacing(4);

                let row = widget::row::with_capacity(2)
                    .push(entry_text)
                    .push(buttons)
                    .spacing(space_xs)
                    .align_y(cosmic::iced::Alignment::Center);

                let row_container =
                    widget::container(widget::mouse_area(row).on_press(Message::SelectIndex(idx)))
                        .width(Length::Fill)
                        .padding(cosmic::iced::Padding {
                            top: 4.0,
                            right: 0.0,
                            bottom: 4.0,
                            left: 8.0,
                        })
                        .class(if is_selected {
                            cosmic::theme::Container::Primary
                        } else {
                            cosmic::theme::Container::Transparent
                        });

                list = list.add(row_container);
            }

            widget::scrollable(list)
                .id(self.scrollable_id.clone())
                .height(Length::Fixed(LIST_HEIGHT))
                .into()
        };

        let page_toggle = widget::row::with_capacity(2)
            .push(all_btn)
            .push(pinned_btn)
            .spacing(space_xs)
            .align_y(cosmic::iced::Alignment::Center);

        // Footer row (unchanged from original)
        let controls = widget::row::with_capacity(3)
            .push(
                widget::button::icon(widget::icon::from_name("edit-clear-symbolic"))
                    .extra_small()
                    .on_press(Message::ClearHistory)
                    .tooltip(fl!("clear-history"))
                    .class(cosmic::theme::Button::Text),
            )
            .push(
                widget::button::icon(widget::icon::from_name("application-menu-symbolic"))
                    .extra_small()
                    .on_press(Message::Settings)
                    .tooltip(fl!("settings"))
                    .class(cosmic::theme::Button::Text),
            )
            .push(
                widget::button::icon(widget::icon::from_name("go-home-symbolic"))
                    .extra_small()
                    .on_press(Message::OpenOverlay)
                    .tooltip(fl!("open-overlay"))
                    .class(cosmic::theme::Button::Text),
            )
            .spacing(space_xs);

        let control = widget::container(controls);
        let mut nav = widget::row::with_capacity(3)
            .push(page_toggle)
            .push(widget::horizontal_space());
        nav = nav.push(control);
        let content = widget::column::with_capacity(3)
            .push(search_bar)
            .push(nav)
            .push(list_content)
            .spacing(space_xs)
            .padding(8);

        self.core.applet.popup_container(content).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        listen_raw(|event, status, _window| {
            if let Event::Keyboard(keyboard::Event::KeyPressed {
                ref key, modifiers, ..
            }) = event
            {
                let config = APPLET_KEYBINDINGS
                    .read()
                    .ok()
                    .and_then(|kb| kb.clone())
                    .unwrap_or_default();

                // Configurable keybindings
                if status == event::Status::Ignored {
                    if config.toggle_pin.matches(key, modifiers) {
                        return Some(Message::TogglePinSelected);
                    }
                    if config.delete_entry.matches(key, modifiers) {
                        return Some(Message::DeleteSelected);
                    }
                }
                if config.tab_all.matches(key, modifiers) {
                    return Some(Message::SetPage(Page::All));
                }
                if config.tab_pinned.matches(key, modifiers) {
                    return Some(Message::SetPage(Page::Pinned));
                }
            }

            // Hardcoded navigation
            match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Escape),
                    ..
                }) if status == event::Status::Ignored => Some(Message::TogglePopup),

                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::ArrowDown),
                    modifiers,
                    ..
                }) if status == event::Status::Ignored
                    && !modifiers.control()
                    && !modifiers.alt()
                    && !modifiers.shift() =>
                {
                    Some(Message::SelectNext)
                }

                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::ArrowUp),
                    modifiers,
                    ..
                }) if status == event::Status::Ignored
                    && !modifiers.control()
                    && !modifiers.alt()
                    && !modifiers.shift() =>
                {
                    Some(Message::SelectPrevious)
                }

                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Enter),
                    ..
                }) if status == event::Status::Ignored => Some(Message::ActivateSelected),

                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Backspace),
                    ..
                }) if status == event::Status::Ignored => Some(Message::SearchBackspace),

                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Character(ref c),
                    modifiers,
                    ..
                }) if status == event::Status::Ignored
                    && !modifiers.control()
                    && !modifiers.alt() =>
                {
                    c.chars().next().map(Message::SearchChar)
                }

                _ => None,
            }
        })
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }
}

async fn fetch_entries() -> Result<Vec<ClipboardEntry>, String> {
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| e.to_string())?;

    let proxy = crate::schema::dbus::ClipboardManagerProxy::builder(&conn)
        .destination(DBUS_NAME)
        .map_err(|e| e.to_string())?
        .path(DBUS_PATH)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    let bytes = proxy.list_entries().await.map_err(|e| e.to_string())?;
    let entries: Vec<ClipboardEntry> =
        OxiCodeCodec::deserialize(&bytes).map_err(|e| e.to_string())?;
    Ok(entries)
}

enum DaemonAction {
    Paste(u64),
    Delete(u64),
    Pin(u64),
    Unpin(u64),
    Clear,
}

async fn call_daemon_action(action: DaemonAction) -> Result<bool, String> {
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| e.to_string())?;

    let proxy = crate::schema::dbus::ClipboardManagerProxy::builder(&conn)
        .destination(DBUS_NAME)
        .map_err(|e| e.to_string())?
        .path(DBUS_PATH)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    match action {
        DaemonAction::Paste(id) => {
            proxy.paste_entry(id).await.map_err(|e| e.to_string())?;
        }
        DaemonAction::Delete(id) => {
            proxy.delete_entry(id).await.map_err(|e| e.to_string())?;
        }
        DaemonAction::Pin(id) => {
            proxy.pin_entry(id).await.map_err(|e| e.to_string())?;
        }
        DaemonAction::Unpin(id) => {
            proxy.unpin_entry(id).await.map_err(|e| e.to_string())?;
        }
        DaemonAction::Clear => {
            proxy.clear().await.map_err(|e| e.to_string())?;
        }
    }
    Ok(true)
}
