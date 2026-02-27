// SPDX-License-Identifier: MPL-2.0

//! COSMIC panel applet mode - mini clipboard manager popup.

use cliphoard_schema::entry::ClipboardEntry;
use cliphoard_schema::{Codec, DBUS_NAME, DBUS_PATH, OxiCodeCodec};
use cosmic::app::{Application, Core, Task};
use cosmic::iced::platform_specific::shell::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{window, Length};
use cosmic::widget;
use cosmic::Element;
use tracing::debug;

const PREVIEW_LEN: usize = 24;
const MAX_ENTRIES: usize = 10;

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(window::Id),
    OpenOverlay,
    ClearHistory,
    Settings,
    EntriesLoaded(Result<Vec<ClipboardEntry>, String>),
    PasteEntry(u64),
    DeleteEntry(u64),
    TogglePin(u64, bool),
    ActionDone(Result<bool, String>),
}

pub struct AppletModel {
    core: Core,
    popup: Option<window::Id>,
    entries: Vec<ClipboardEntry>,
    error: Option<String>,
}

impl Application for AppletModel {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "com.github.al_ula.Cliphoard.Applet";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, cosmic::app::Task<Message>) {
        (
            Self {
                core,
                popup: None,
                entries: Vec::new(),
                error: None,
            },
            cosmic::app::Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> cosmic::app::Task<Message> {
        match message {
            Message::TogglePopup => {
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                } else {
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
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
            Message::OpenOverlay => {
                std::process::Command::new("cliphoard").spawn().ok();
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
            }
            Message::ClearHistory => {
                return Task::perform(
                    call_daemon_action(DaemonAction::Clear),
                    |r| cosmic::Action::App(Message::ActionDone(r)),
                );
            }
            Message::Settings => {
                debug!("Settings button pressed (placeholder)");
            }
            Message::EntriesLoaded(result) => match result {
                Ok(entries) => {
                    self.entries = entries.into_iter().take(MAX_ENTRIES).collect();
                    self.error = None;
                }
                Err(e) => {
                    self.error = Some(e);
                    self.entries.clear();
                }
            },
            Message::PasteEntry(id) => {
                if let Some(p) = self.popup.take() {
                    return Task::batch(vec![
                        destroy_popup(p),
                        Task::perform(
                            call_daemon_action(DaemonAction::Paste(id)),
                            |r| cosmic::Action::App(Message::ActionDone(r)),
                        ),
                    ]);
                }
                return Task::perform(
                    call_daemon_action(DaemonAction::Paste(id)),
                    |r| cosmic::Action::App(Message::ActionDone(r)),
                );
            }
            Message::DeleteEntry(id) => {
                return Task::perform(
                    call_daemon_action(DaemonAction::Delete(id)),
                    |r| cosmic::Action::App(Message::ActionDone(r)),
                );
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
        }
        cosmic::app::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let suggested_size = self.core.applet.suggested_size(true);
        self.core
            .applet
            .icon_button_from_handle(
                widget::icon::from_name("com.github.al-ula.Cliphoard")
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

        let list_content: Element<_> = if let Some(ref err) = self.error {
            widget::text::body(err.as_str()).into()
        } else if self.entries.is_empty() {
            widget::text::body("Clipboard is empty").into()
        } else {
            let mut list = widget::list_column().padding(0).spacing(4);

            for entry in &self.entries {
                let id = entry.id.0;
                let preview = entry.preview(PREVIEW_LEN);
                let pinned = entry.pinned;

                let entry_text = widget::text(preview)
                    .width(Length::Fill)
                    .wrapping(cosmic::iced::widget::text::Wrapping::None)
                    .shaping(cosmic::iced::widget::text::Shaping::Basic);

                let buttons = widget::row::with_capacity(2)
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

                let text_container = widget::container(entry_text)
                    .width(Length::Fill)
                    .clip(true);

                let row = widget::row::with_capacity(2)
                    .push(text_container)
                    .push(buttons)
                    .spacing(space_xs)
                    .align_y(cosmic::iced::Alignment::Center);

                let row_container = widget::container(
                        widget::mouse_area(row)
                            .on_press(Message::PasteEntry(id)),
                    )
                    .width(Length::Fill)
                    .padding(cosmic::iced::Padding { top: 4.0, right: 0.0, bottom: 4.0, left: 8.0 })
                    .class(cosmic::theme::Container::Transparent);

                list = list.add(row_container);
            }

            widget::scrollable(list).height(Length::Fixed(280.0)).into()
        };

        let controls = widget::row::with_capacity(3)
            .push(
                widget::button::text("Open")
                    .on_press(Message::OpenOverlay),
            )
            .push(
                widget::button::text("Clear")
                    .on_press(Message::ClearHistory),
            )
            .push(
                widget::button::text("Settings")
                    .on_press(Message::Settings),
            )
            .spacing(space_xs);

        let controls_centered = widget::container(controls)
            .width(Length::Fill)
            .align_x(cosmic::iced::Alignment::Center);

        let content = widget::column::with_capacity(2)
            .push(list_content)
            .push(controls_centered)
            .spacing(space_xs)
            .padding(8);

        self.core.applet.popup_container(content).into()
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

    let proxy = cliphoard_schema::dbus::ClipboardManagerProxy::builder(&conn)
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

    let proxy = cliphoard_schema::dbus::ClipboardManagerProxy::builder(&conn)
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
