// SPDX-License-Identifier: MPL-2.0

//! Layer-shell overlay for the clipboard manager.

use crate::config::{Config, KeyBinding, KeyBindingExt, KeybindingsConfig};
use crate::fl;
use crate::schema::entry::ClipboardEntry;
use crate::schema::{Codec, DBUS_NAME, DBUS_PATH, OxiCodeCodec};
use cosmic::iced::event::{self, Event, listen_raw};
use cosmic::iced::futures::SinkExt;
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::SctkLayerSurfaceSettings;
use cosmic::iced::platform_specific::shell::commands::layer_surface::{
    Anchor, KeyboardInteractivity, Layer, destroy_layer_surface, get_layer_surface,
};
use cosmic::iced::widget::scrollable::{AbsoluteOffset, scroll_to};
use cosmic::iced::{Color, Length, Subscription, Task, window};
use cosmic::iced_core::keyboard::{self, Modifiers, key::Named};
use cosmic::iced_core::widget::Id;
use cosmic::iced_runtime::Appearance;
use cosmic::widget;
use cosmic::{Element, Theme};
use std::str::FromStr;

const PREVIEW_LEN: usize = 32;
const LIST_PANEL_WIDTH: f32 = 400.0;
const PREVIEW_PANEL_WIDTH: f32 = 350.0;
const CARD_HEIGHT: f32 = 500.0;

/// Initial view to open when the overlay launches.
static INITIAL_VIEW: std::sync::OnceLock<View> = std::sync::OnceLock::new();

/// Keybinding config, runtime-updatable via settings.
static KEYBINDINGS: std::sync::RwLock<Option<Config>> = std::sync::RwLock::new(None);

/// Current keybinding capture state (which keybinding is being captured).
static CAPTURING: std::sync::RwLock<bool> = std::sync::RwLock::new(false);

pub fn run_overlay_with_view(view: View) -> cosmic::iced::Result {
    INITIAL_VIEW.set(view).ok();
    cosmic::iced::daemon(
        OverlayState::title,
        OverlayState::update,
        OverlayState::view,
    )
    .subscription(OverlayState::subscription)
    .theme(OverlayState::theme)
    .style(OverlayState::style)
    .run_with(OverlayState::new)
}

pub fn run_overlay() -> cosmic::iced::Result {
    run_overlay_with_view(View::Main)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    All,
    Pinned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Main,
    Settings,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturingKeybinding {
    None,
    TogglePin,
    TogglePreview,
    DeleteEntry,
    TabAll,
    TabPinned,
}

struct OverlayState {
    surface_id: Option<window::Id>,
    visible: bool,
    entries: Vec<ClipboardEntry>,
    search_query: String,
    error: Option<String>,
    page: Page,
    selected_index: Option<usize>,
    scrollable_id: Id,
    search_input_id: widget::Id,
    view: View,
    show_preview: bool,
    config: Config,
    settings_limits: SettingsLimits,
    settings_keybindings: SettingsKeybindings,
    capturing_keybinding: CapturingKeybinding,
    keybinding_errors: KeybindingErrors,
    show_tips: bool,
    show_daemon_notice: bool,
    daemon_notice_dismissed: bool,
}

#[derive(Debug, Clone)]
struct SettingsLimits {
    max_unpinned: String,
    max_pinned: String,
    max_entry_size: String,
}

impl Default for SettingsLimits {
    fn default() -> Self {
        Self {
            max_unpinned: crate::schema::DEFAULT_MAX_UNPINNED.to_string(),
            max_pinned: crate::schema::DEFAULT_MAX_PINNED.to_string(),
            max_entry_size: (crate::schema::DEFAULT_MAX_ENTRY_SIZE / (1024 * 1024)).to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct SettingsKeybindings {
    toggle_pin: String,
    toggle_preview: String,
    delete_entry: String,
    tab_all: String,
    tab_pinned: String,
}

#[derive(Debug, Clone, Default)]
struct KeybindingErrors {
    toggle_pin: Option<String>,
    toggle_preview: Option<String>,
    delete_entry: Option<String>,
    tab_all: Option<String>,
    tab_pinned: Option<String>,
}

impl KeybindingErrors {
    fn has_errors(&self) -> bool {
        self.toggle_pin.is_some()
            || self.toggle_preview.is_some()
            || self.delete_entry.is_some()
            || self.tab_all.is_some()
            || self.tab_pinned.is_some()
    }
}

impl From<&Config> for SettingsKeybindings {
    fn from(config: &Config) -> Self {
        Self {
            toggle_pin: config.toggle_pin.to_string(),
            toggle_preview: config.toggle_preview.to_string(),
            delete_entry: config.delete_entry.to_string(),
            tab_all: config.tab_all.to_string(),
            tab_pinned: config.tab_pinned.to_string(),
        }
    }
}

impl SettingsKeybindings {
    fn to_config(&self) -> KeybindingsConfig {
        KeybindingsConfig {
            toggle_pin: KeyBinding::from_str(&self.toggle_pin).unwrap_or_default(),
            toggle_preview: KeyBinding::from_str(&self.toggle_preview).unwrap_or_default(),
            delete_entry: KeyBinding::from_str(&self.delete_entry).unwrap_or_default(),
            tab_all: KeyBinding::from_str(&self.tab_all).unwrap_or_default(),
            tab_pinned: KeyBinding::from_str(&self.tab_pinned).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Hide,
    EntriesLoaded(Result<Vec<ClipboardEntry>, String>),
    SearchChanged(String),
    SearchChar(char),
    SearchBackspace,
    CopyEntry(u64),
    DeleteEntry(u64),
    TogglePin(u64, bool),
    SelectIndex(usize),
    ClearHistory,
    ActionDone(Result<bool, String>),
    RefreshEntries,
    SetPage(Page),
    LayerUnfocused,
    NoOp,
    SelectNext,
    SelectPrevious,
    ActivateSelected,
    DeleteSelected,
    TogglePinSelected,
    TogglePreview,
    SetView(View),
    SettingsMaxUnpinnedChanged(String),
    SettingsMaxPinnedChanged(String),
    SettingsMaxEntrySizeChanged(String),
    SettingsKeybindingChanged(String, String),
    StartCaptureKeybinding(CapturingKeybinding),
    KeybindingCaptured(String),
    CancelCaptureKeybinding,
    SaveSettings,
    ResetSettings,
    SettingsSaved(Result<(), String>),
    OpenUrl(String),
    DismissTips,
    DismissDaemonNotice,
}

impl std::fmt::Debug for OverlayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OverlayState")
            .field("visible", &self.visible)
            .finish()
    }
}

impl OverlayState {
    fn new() -> (Self, Task<Message>) {
        let config = crate::config::load_config();
        let settings_keybindings = SettingsKeybindings::from(&config);
        let mut state = OverlayState {
            surface_id: None,
            visible: false,
            entries: Vec::new(),
            search_query: String::new(),
            error: None,
            page: Page::All,
            selected_index: None,
            scrollable_id: Id::unique(),
            search_input_id: widget::Id::unique(),
            view: INITIAL_VIEW.get().copied().unwrap_or(View::Main),
            show_preview: true,
            config: config.clone(),
            settings_limits: SettingsLimits::default(),
            settings_keybindings,
            capturing_keybinding: CapturingKeybinding::None,
            keybinding_errors: KeybindingErrors::default(),
            show_tips: crate::config::is_first_launch(),
            show_daemon_notice: false,
            daemon_notice_dismissed: false,
        };
        if let Ok(mut kb) = KEYBINDINGS.write() {
            *kb = Some(state.config.clone());
        }
        let task = state.show();
        (state, task)
    }

    fn title(&self, _id: window::Id) -> String {
        String::from("Cliphoard")
    }

    fn theme(&self, _id: window::Id) -> Theme {
        cosmic::theme::system_preference()
    }

    fn style(&self, theme: &Theme) -> Appearance {
        // Transparent background so the layer surface acts as a scrim.
        let cosmic = theme.cosmic();
        Appearance {
            background_color: Color::TRANSPARENT,
            text_color: cosmic.on_bg_color().into(),
            icon_color: cosmic.on_bg_color().into(),
        }
    }

    fn show(&mut self) -> Task<Message> {
        if self.visible {
            return Task::none();
        }
        let id = window::Id::unique();
        self.surface_id = Some(id);
        self.visible = true;
        self.search_query.clear();
        self.page = Page::All;
        self.selected_index = None;

        Task::batch(vec![
            get_layer_surface(SctkLayerSurfaceSettings {
                id,
                layer: Layer::Top,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                anchor: Anchor::TOP
                    .union(Anchor::BOTTOM)
                    .union(Anchor::LEFT)
                    .union(Anchor::RIGHT),
                namespace: String::from("cliphoard-overlay"),
                ..Default::default()
            }),
            Task::perform(fetch_entries(), Message::EntriesLoaded),
            widget::text_input::focus(self.search_input_id.clone()),
        ])
    }

    fn hide(&mut self) -> Task<Message> {
        if !self.visible {
            return Task::none();
        }
        self.visible = false;
        if let Some(id) = self.surface_id.take() {
            destroy_layer_surface(id)
        } else {
            Task::none()
        }
    }

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

    fn filtered_entries_count(&self) -> usize {
        self.filtered_entries().len()
    }

    fn selected_entry_id(&self) -> Option<u64> {
        let filtered = self.filtered_entries();
        self.selected_index
            .and_then(|idx| filtered.get(idx))
            .map(|e| e.id.0)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NoOp => {
                return Task::none();
            }
            Message::Hide | Message::LayerUnfocused => {
                std::process::exit(0);
            }
            Message::EntriesLoaded(result) => match result {
                Ok(entries) => {
                    let was_empty = self.entries.is_empty();
                    self.entries = entries;
                    self.error = None;
                    self.show_daemon_notice = false;
                    if was_empty {
                        self.selected_index = Some(0);
                    } else if let Some(idx) = self.selected_index {
                        let count = self.filtered_entries_count();
                        if count == 0 {
                            self.selected_index = None;
                        } else if idx >= count {
                            self.selected_index = Some(count - 1);
                        }
                    }
                }
                Err(e) => {
                    if !self.daemon_notice_dismissed {
                        self.show_daemon_notice = true;
                    }
                    self.error = Some(e);
                }
            },
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.selected_index = Some(0);
            }
            Message::SearchChar(c) => {
                self.search_query.push(c);
                self.selected_index = None;
                return widget::text_input::focus(self.search_input_id.clone());
            }
            Message::SearchBackspace => {
                self.search_query.pop();
                self.selected_index = None;
                return widget::text_input::focus(self.search_input_id.clone());
            }
            Message::SetPage(page) => {
                self.page = page;
                self.selected_index = Some(0);
                return widget::text_input::focus(widget::Id::unique());
            }
            Message::RefreshEntries => {
                if self.visible {
                    return Task::perform(fetch_entries(), Message::EntriesLoaded);
                }
            }
            Message::CopyEntry(id) => {
                let hide = self.hide();
                return Task::batch(vec![
                    Task::perform(
                        call_daemon_action(DaemonAction::Paste(id)),
                        Message::ActionDone,
                    ),
                    hide,
                ]);
            }
            Message::SelectIndex(idx) => {
                self.selected_index = Some(idx);
                let unfocus = widget::text_input::focus(widget::Id::unique());
                const ITEM_HEIGHT: f32 = 40.0;
                let offset = AbsoluteOffset {
                    x: 0.0,
                    y: idx as f32 * ITEM_HEIGHT,
                };
                return Task::batch(vec![unfocus, scroll_to(self.scrollable_id.clone(), offset)]);
            }
            Message::DeleteEntry(id) => {
                let filtered_count = self.filtered_entries_count();
                if let Some(idx) = self.selected_index
                    && idx >= filtered_count.saturating_sub(1)
                {
                    self.selected_index = if filtered_count > 1 {
                        Some(filtered_count - 2)
                    } else {
                        Some(0)
                    };
                }
                return Task::perform(
                    call_daemon_action(DaemonAction::Delete(id)),
                    Message::ActionDone,
                );
            }
            Message::TogglePin(id, pin) => {
                return Task::perform(
                    call_daemon_action(if pin {
                        DaemonAction::Pin(id)
                    } else {
                        DaemonAction::Unpin(id)
                    }),
                    Message::ActionDone,
                );
            }
            Message::ClearHistory => {
                return Task::perform(call_daemon_action(DaemonAction::Clear), Message::ActionDone);
            }
            Message::ActionDone(result) => {
                match result {
                    Ok(true) => {
                        std::process::exit(0);
                    }
                    Err(ref e) if e.contains("pin_limit_reached") => {
                        self.error = Some(fl!("pin-limit-reached"));
                        return Task::perform(fetch_entries(), Message::EntriesLoaded);
                    }
                    _ => {}
                }
                return Task::perform(fetch_entries(), Message::EntriesLoaded);
            }
            Message::SelectNext => {
                const ITEM_HEIGHT: f32 = 40.0;
                let filtered_count = self.filtered_entries_count();
                if filtered_count > 0 {
                    let new_index = match self.selected_index {
                        None => 0,
                        Some(i) => (i + 1).min(filtered_count - 1),
                    };
                    self.selected_index = Some(new_index);
                    let offset = AbsoluteOffset {
                        x: 0.0,
                        y: new_index as f32 * ITEM_HEIGHT,
                    };
                    let unfocus = widget::text_input::focus(widget::Id::unique());
                    return Task::batch(vec![
                        unfocus,
                        scroll_to(self.scrollable_id.clone(), offset),
                    ]);
                }
            }
            Message::SelectPrevious => {
                const ITEM_HEIGHT: f32 = 40.0;
                match self.selected_index {
                    None => {}
                    Some(0) => {
                        self.selected_index = None;
                        return widget::text_input::focus(self.search_input_id.clone());
                    }
                    Some(i) => {
                        let new_index = i.saturating_sub(1);
                        self.selected_index = Some(new_index);
                        let offset = AbsoluteOffset {
                            x: 0.0,
                            y: new_index as f32 * ITEM_HEIGHT,
                        };
                        return scroll_to(self.scrollable_id.clone(), offset);
                    }
                }
            }
            Message::ActivateSelected => {
                let id = if let Some(id) = self.selected_entry_id() {
                    Some(id)
                } else if self.selected_index.is_none() {
                    self.filtered_entries().first().map(|e| e.id.0)
                } else {
                    None
                };
                if let Some(id) = id {
                    let hide = self.hide();
                    return Task::batch(vec![
                        Task::perform(
                            call_daemon_action(DaemonAction::Paste(id)),
                            Message::ActionDone,
                        ),
                        hide,
                    ]);
                }
            }
            Message::DeleteSelected => {
                if let Some(id) = self.selected_entry_id() {
                    let filtered_count = self.filtered_entries_count();
                    if let Some(idx) = self.selected_index
                        && idx >= filtered_count.saturating_sub(1)
                    {
                        self.selected_index = if filtered_count > 1 {
                            Some(filtered_count - 2)
                        } else {
                            Some(0)
                        };
                    }
                    return Task::perform(
                        call_daemon_action(DaemonAction::Delete(id)),
                        Message::ActionDone,
                    );
                }
            }
            Message::SetView(view) => {
                self.view = view;
            }
            Message::TogglePreview => {
                self.show_preview = !self.show_preview;
            }
            Message::TogglePinSelected => {
                if let Some(id) = self.selected_entry_id()
                    && let Some(entry) = self.entries.iter().find(|e| e.id.0 == id)
                {
                    let pinned = entry.pinned;
                    return Task::perform(
                        call_daemon_action(if pinned {
                            DaemonAction::Unpin(id)
                        } else {
                            DaemonAction::Pin(id)
                        }),
                        Message::ActionDone,
                    );
                }
            }
            Message::SettingsMaxUnpinnedChanged(value) => {
                self.settings_limits.max_unpinned = value;
            }
            Message::SettingsMaxPinnedChanged(value) => {
                self.settings_limits.max_pinned = value;
            }
            Message::SettingsMaxEntrySizeChanged(value) => {
                self.settings_limits.max_entry_size = value;
            }
            Message::SettingsKeybindingChanged(name, value) => match name.as_str() {
                "toggle_pin" => {
                    self.keybinding_errors.toggle_pin = KeyBinding::from_str(&value).err();
                    self.settings_keybindings.toggle_pin = value;
                }
                "toggle_preview" => {
                    self.keybinding_errors.toggle_preview = KeyBinding::from_str(&value).err();
                    self.settings_keybindings.toggle_preview = value;
                }
                "delete_entry" => {
                    self.keybinding_errors.delete_entry = KeyBinding::from_str(&value).err();
                    self.settings_keybindings.delete_entry = value;
                }
                "tab_all" => {
                    self.keybinding_errors.tab_all = KeyBinding::from_str(&value).err();
                    self.settings_keybindings.tab_all = value;
                }
                "tab_pinned" => {
                    self.keybinding_errors.tab_pinned = KeyBinding::from_str(&value).err();
                    self.settings_keybindings.tab_pinned = value;
                }
                _ => {}
            },
            Message::StartCaptureKeybinding(which) => {
                self.capturing_keybinding = which;
                if let Ok(mut c) = CAPTURING.write() {
                    *c = which != CapturingKeybinding::None;
                }
            }
            Message::KeybindingCaptured(keybinding_str) => {
                let which = self.capturing_keybinding;
                self.capturing_keybinding = CapturingKeybinding::None;
                if let Ok(mut c) = CAPTURING.write() {
                    *c = false;
                }
                match which {
                    CapturingKeybinding::TogglePin => {
                        self.keybinding_errors.toggle_pin =
                            KeyBinding::from_str(&keybinding_str).err();
                        self.settings_keybindings.toggle_pin = keybinding_str;
                    }
                    CapturingKeybinding::TogglePreview => {
                        self.keybinding_errors.toggle_preview =
                            KeyBinding::from_str(&keybinding_str).err();
                        self.settings_keybindings.toggle_preview = keybinding_str;
                    }
                    CapturingKeybinding::DeleteEntry => {
                        self.keybinding_errors.delete_entry =
                            KeyBinding::from_str(&keybinding_str).err();
                        self.settings_keybindings.delete_entry = keybinding_str;
                    }
                    CapturingKeybinding::TabAll => {
                        self.keybinding_errors.tab_all =
                            KeyBinding::from_str(&keybinding_str).err();
                        self.settings_keybindings.tab_all = keybinding_str;
                    }
                    CapturingKeybinding::TabPinned => {
                        self.keybinding_errors.tab_pinned =
                            KeyBinding::from_str(&keybinding_str).err();
                        self.settings_keybindings.tab_pinned = keybinding_str;
                    }
                    CapturingKeybinding::None => {}
                }
            }
            Message::CancelCaptureKeybinding => {
                self.capturing_keybinding = CapturingKeybinding::None;
                if let Ok(mut c) = CAPTURING.write() {
                    *c = false;
                }
            }
            Message::SaveSettings => {
                if self.keybinding_errors.has_errors() {
                    self.error = Some(fl!("settings-invalid-keybindings"));
                    return Task::none();
                }

                let max_unpinned = self.settings_limits.max_unpinned.parse().unwrap_or(500);
                let max_pinned = self.settings_limits.max_pinned.parse().unwrap_or(50);
                let max_entry_size =
                    self.settings_limits.max_entry_size.parse().unwrap_or(10) * 1024 * 1024;

                let keybindings_config = self.settings_keybindings.to_config();
                if let Err(e) = crate::config::save_keybindings(&keybindings_config) {
                    self.error = Some(e);
                    return Task::none();
                }

                self.config = keybindings_config.clone();
                if let Ok(mut kb) = KEYBINDINGS.write() {
                    *kb = Some(self.config.clone());
                }

                return Task::perform(
                    update_daemon_config(max_unpinned, max_pinned, max_entry_size),
                    Message::SettingsSaved,
                );
            }
            Message::ResetSettings => {
                self.settings_limits = SettingsLimits::default();
                let default_config = KeybindingsConfig::default();
                self.settings_keybindings = SettingsKeybindings::from(&default_config);
                self.keybinding_errors = KeybindingErrors::default();
            }
            Message::SettingsSaved(result) => {
                if let Err(e) = result {
                    self.error = Some(e);
                }
            }
            Message::OpenUrl(url) => {
                let _ = open::that(&url);
            }
            Message::DismissTips => {
                self.show_tips = false;
                crate::config::mark_first_launch_done();
            }
            Message::DismissDaemonNotice => {
                self.show_daemon_notice = false;
                self.daemon_notice_dismissed = true;
            }
        }
        Task::none()
    }

    fn tips_view(&self) -> Element<'_, Message> {
        let space_s = cosmic::theme::spacing().space_s;
        let space_xs = cosmic::theme::spacing().space_xs;

        let title = widget::text::title4(fl!("tips-title"));

        let tips = widget::column::with_capacity(6)
            .push(widget::text::body(fl!("tips-shortcut")))
            .push(widget::text::body(fl!(
                "tips-pin",
                keybinding = self.config.toggle_pin.to_string()
            )))
            .push(widget::text::body(fl!(
                "tips-preview",
                keybinding = self.config.toggle_preview.to_string()
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
            .spacing(space_xs);

        let dismiss = widget::button::text(fl!("tips-dismiss"))
            .on_press(Message::DismissTips)
            .class(cosmic::theme::Button::Suggested);

        let content = widget::column::with_capacity(3)
            .push(title)
            .push(tips)
            .push(
                widget::row::with_capacity(1)
                    .push(widget::horizontal_space())
                    .push(dismiss),
            )
            .spacing(space_s)
            .padding(space_s);

        widget::container(content)
            .width(Length::Fixed(LIST_PANEL_WIDTH))
            .class(cosmic::theme::Container::Primary)
            .into()
    }

    fn daemon_notice_view(&self) -> Element<'_, Message> {
        let space_s = cosmic::theme::spacing().space_s;
        let space_xs = cosmic::theme::spacing().space_xs;

        let title = widget::text::title4(fl!("daemon-notice-title"));

        let body = widget::text::body(fl!("daemon-notice-body"));

        let service_cmd = widget::container(
            widget::text::body(fl!("daemon-notice-service")).font(cosmic::iced::Font::MONOSPACE),
        )
        .padding([4, 8])
        .class(cosmic::theme::Container::Primary);

        let or_label = widget::text::body(fl!("daemon-notice-or"));

        let manual_cmd = widget::container(
            widget::text::body(fl!("daemon-notice-command")).font(cosmic::iced::Font::MONOSPACE),
        )
        .padding([4, 8])
        .class(cosmic::theme::Container::Primary);

        let dismiss = widget::button::text(fl!("daemon-notice-dismiss"))
            .on_press(Message::DismissDaemonNotice)
            .class(cosmic::theme::Button::Suggested);

        let content = widget::column::with_capacity(6)
            .push(title)
            .push(body)
            .push(service_cmd)
            .push(or_label)
            .push(manual_cmd)
            .push(
                widget::row::with_capacity(1)
                    .push(widget::horizontal_space())
                    .push(dismiss),
            )
            .spacing(space_xs)
            .padding(space_s);

        widget::container(content)
            .width(Length::Fixed(LIST_PANEL_WIDTH))
            .class(cosmic::theme::Container::Primary)
            .into()
    }

    fn keybinding_row<'a>(
        &'a self,
        label: String,
        value: &'a str,
        error: Option<&'a str>,
        which: CapturingKeybinding,
    ) -> Element<'a, Message> {
        let space_xs = cosmic::theme::spacing().space_xs;

        let is_capturing = self.capturing_keybinding == which;

        let text_input = widget::text_input("", value)
            .width(Length::Fixed(140.0))
            .on_input(move |v| {
                let name = match which {
                    CapturingKeybinding::TogglePin => "toggle_pin",
                    CapturingKeybinding::TogglePreview => "toggle_preview",
                    CapturingKeybinding::DeleteEntry => "delete_entry",
                    CapturingKeybinding::TabAll => "tab_all",
                    CapturingKeybinding::TabPinned => "tab_pinned",
                    CapturingKeybinding::None => "",
                };
                Message::SettingsKeybindingChanged(name.to_string(), v)
            });

        let capture_text = if is_capturing {
            fl!("settings-capturing")
        } else {
            fl!("settings-capture")
        };

        let capture_btn = widget::button::text(capture_text)
            .on_press(Message::StartCaptureKeybinding(which))
            .class(if is_capturing {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            });

        let control = widget::row::with_capacity(2)
            .push(text_input)
            .push(capture_btn)
            .spacing(space_xs)
            .align_y(cosmic::iced::Alignment::Center);

        if let Some(err) = error {
            widget::settings::item::builder(label)
                .description(err)
                .control(control)
                .into()
        } else {
            widget::settings::item(label, control).into()
        }
    }

    fn view(&self, _id: window::Id) -> Element<'_, Message> {
        if !self.visible {
            return widget::text("").into();
        }

        let space_s = cosmic::theme::spacing().space_s;
        let space_xs = cosmic::theme::spacing().space_xs;

        let search_focused = self.selected_index.is_none();
        let search_bar = widget::text_input(fl!("search-placeholder"), &self.search_query)
            .id(self.search_input_id.clone())
            .on_input(Message::SearchChanged)
            .width(Length::Fill);
        let search_bar: Element<'_, Message> = if search_focused {
            widget::container(search_bar)
                .padding([2, 4])
                .class(cosmic::theme::Container::Primary)
                .into()
        } else {
            search_bar.into()
        };

        let all_btn = widget::button::text(fl!("all-entries"))
            .on_press(Message::SetPage(Page::All))
            .class(if self.page == Page::All {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            });

        let pinned_btn = widget::button::text(fl!("pinned-entries"))
            .on_press(Message::SetPage(Page::Pinned))
            .class(if self.page == Page::Pinned {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            });

        let mut page_toggle = widget::row::with_capacity(3).push(all_btn).push(pinned_btn);

        page_toggle = page_toggle.push(widget::horizontal_space());

        if self.page == Page::All {
            page_toggle = page_toggle.push(
                widget::button::icon(widget::icon::from_name("edit-clear-symbolic"))
                    .on_press(Message::ClearHistory)
                    .class(cosmic::theme::Button::Destructive),
            );
        }

        page_toggle = page_toggle.push(
            widget::button::icon(widget::icon::from_name("view-dual-symbolic"))
                .on_press(Message::TogglePreview)
                .class(if self.show_preview {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Standard
                }),
        );

        let page_toggle = page_toggle
            .spacing(space_xs)
            .align_y(cosmic::iced::Alignment::Center);

        let filtered = self.filtered_entries();
        let entries_count = filtered.len();

        let body: Element<_> = if let Some(ref err) = self.error {
            widget::text::body(err.as_str()).into()
        } else if filtered.is_empty() {
            widget::text::body(fl!("clipboard-empty")).into()
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
                .height(Length::Fill)
                .into()
        };

        let settings_button = match self.view {
            View::Settings => widget::button::icon(widget::icon::from_name("go-home-symbolic"))
                .extra_small()
                .on_press(Message::SetView(View::Main))
                .class(cosmic::theme::Button::Text),
            _ => widget::button::icon(widget::icon::from_name("emblem-system-symbolic"))
                .extra_small()
                .on_press(Message::SetView(View::Settings))
                .class(cosmic::theme::Button::Text),
        };

        let about_button = match self.view {
            View::About => widget::button::icon(widget::icon::from_name("go-home-symbolic"))
                .extra_small()
                .on_press(Message::SetView(View::Main))
                .class(cosmic::theme::Button::Text),
            _ => widget::button::icon(widget::icon::from_name("help-about-symbolic"))
                .extra_small()
                .on_press(Message::SetView(View::About))
                .class(cosmic::theme::Button::Text),
        };

        let mut footer_row = widget::row::with_capacity(4);
        if self.view == View::Main {
            footer_row = footer_row.push(widget::text::caption(format!("{entries_count} entries")));
        }
        footer_row = footer_row
            .push(widget::horizontal_space())
            .push(about_button)
            .push(settings_button)
            .align_y(cosmic::iced::Alignment::Center);

        let footer = widget::container(footer_row).padding([space_s, 0, 0, 0]);

        let main_row: Element<'_, Message> = match self.view {
            View::Main => {
                let list_col = widget::column::with_capacity(3)
                    .push(search_bar)
                    .push(page_toggle)
                    .push(body)
                    .spacing(space_s);

                let list_panel = widget::container(list_col)
                    .width(Length::Fixed(LIST_PANEL_WIDTH))
                    .height(Length::Fill);

                let preview_content: Element<'_, Message> =
                    if let Some(idx) = self.selected_index {
                        if let Some(entry) = filtered.get(idx) {
                            if entry.mime.is_text() {
                                let text = entry.as_text().unwrap_or("");
                                widget::scrollable(
                                    widget::container(widget::text(text).wrapping(
                                        cosmic::iced::widget::text::Wrapping::WordOrGlyph,
                                    ))
                                    .padding(space_s),
                                )
                                .height(Length::Fill)
                                .into()
                            } else if entry.mime.is_image() {
                                let image_widget: Element<'_, Message> =
                                    if entry.mime == crate::schema::MimeType::ImageSvg {
                                        widget::svg(cosmic::widget::svg::Handle::from_memory(
                                            entry.data.clone(),
                                        ))
                                        .width(Length::Fill)
                                        .into()
                                    } else {
                                        widget::image(cosmic::widget::image::Handle::from_bytes(
                                            entry.data.clone(),
                                        ))
                                        .width(Length::Fill)
                                        .into()
                                    };
                                widget::container(image_widget)
                                    .width(Length::Fill)
                                    .height(Length::Fill)
                                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                                    .align_y(cosmic::iced::alignment::Vertical::Center)
                                    .into()
                            } else {
                                let mime = &entry.mime;
                                let len = entry.data.len();
                                widget::container(widget::text(format!(
                                    "[binary: {mime}, {len} bytes]"
                                )))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .align_x(cosmic::iced::alignment::Horizontal::Center)
                                .align_y(cosmic::iced::alignment::Vertical::Center)
                                .into()
                            }
                        } else {
                            widget::container(widget::text(fl!("preview-placeholder")))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .align_x(cosmic::iced::alignment::Horizontal::Center)
                                .align_y(cosmic::iced::alignment::Vertical::Center)
                                .into()
                        }
                    } else {
                        widget::container(widget::text(fl!("preview-placeholder")))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .align_x(cosmic::iced::alignment::Horizontal::Center)
                            .align_y(cosmic::iced::alignment::Vertical::Center)
                            .into()
                    };

                let preview_panel = widget::container(preview_content)
                    .width(Length::Fixed(PREVIEW_PANEL_WIDTH))
                    .height(Length::Fill)
                    .padding(space_s);

                if self.show_preview {
                    let divider = widget::container(
                        widget::container(widget::horizontal_space())
                            .width(Length::Fixed(1.0))
                            .height(Length::Fill)
                            .class(cosmic::theme::Container::Primary),
                    )
                    .padding([0, 4]);

                    widget::row::with_capacity(3)
                        .push(list_panel)
                        .push(divider)
                        .push(preview_panel)
                        .into()
                } else {
                    widget::row::with_capacity(1).push(list_panel).into()
                }
            }
            View::Settings => {
                let limits_section = widget::settings::section()
                    .title(fl!("settings-limits"))
                    .add(widget::settings::item(
                        fl!("settings-max-unpinned"),
                        widget::text_input("", &self.settings_limits.max_unpinned)
                            .width(Length::Fixed(100.0))
                            .on_input(Message::SettingsMaxUnpinnedChanged),
                    ))
                    .add(widget::settings::item(
                        fl!("settings-max-pinned"),
                        widget::text_input("", &self.settings_limits.max_pinned)
                            .width(Length::Fixed(100.0))
                            .on_input(Message::SettingsMaxPinnedChanged),
                    ))
                    .add(widget::settings::item(
                        fl!("settings-max-entry-size"),
                        widget::text_input("", &self.settings_limits.max_entry_size)
                            .width(Length::Fixed(100.0))
                            .on_input(Message::SettingsMaxEntrySizeChanged),
                    ));

                let keybindings_section = widget::settings::section()
                    .title(fl!("settings-keybindings"))
                    .add(self.keybinding_row(
                        fl!("settings-kb-toggle-pin"),
                        &self.settings_keybindings.toggle_pin,
                        self.keybinding_errors.toggle_pin.as_deref(),
                        CapturingKeybinding::TogglePin,
                    ))
                    .add(self.keybinding_row(
                        fl!("settings-kb-toggle-preview"),
                        &self.settings_keybindings.toggle_preview,
                        self.keybinding_errors.toggle_preview.as_deref(),
                        CapturingKeybinding::TogglePreview,
                    ))
                    .add(self.keybinding_row(
                        fl!("settings-kb-delete"),
                        &self.settings_keybindings.delete_entry,
                        self.keybinding_errors.delete_entry.as_deref(),
                        CapturingKeybinding::DeleteEntry,
                    ))
                    .add(self.keybinding_row(
                        fl!("settings-kb-tab-all"),
                        &self.settings_keybindings.tab_all,
                        self.keybinding_errors.tab_all.as_deref(),
                        CapturingKeybinding::TabAll,
                    ))
                    .add(self.keybinding_row(
                        fl!("settings-kb-tab-pinned"),
                        &self.settings_keybindings.tab_pinned,
                        self.keybinding_errors.tab_pinned.as_deref(),
                        CapturingKeybinding::TabPinned,
                    ));

                let has_errors = self.keybinding_errors.has_errors();
                let save_button = if has_errors {
                    widget::button::text(fl!("settings-save"))
                        .class(cosmic::theme::Button::Standard)
                } else {
                    widget::button::text(fl!("settings-save"))
                        .on_press(Message::SaveSettings)
                        .class(cosmic::theme::Button::Suggested)
                };

                let buttons = widget::row::with_capacity(2)
                    .push(widget::horizontal_space())
                    .push(
                        widget::row::with_capacity(2)
                            .push(
                                widget::button::text(fl!("settings-reset"))
                                    .on_press(Message::ResetSettings)
                                    .class(cosmic::theme::Button::Destructive),
                            )
                            .push(save_button)
                            .spacing(space_s),
                    );

                let settings_content = widget::settings::view_column(vec![
                    limits_section.into(),
                    keybindings_section.into(),
                    buttons.into(),
                ]);

                widget::container(widget::scrollable(settings_content).height(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(space_s)
                    .into()
            }
            View::About => {
                // App icon
                let icon = widget::icon::from_name("com.github.al_ula.Cliphoard").size(64);

                // App name
                let app_name = widget::text::title3(fl!("app-title"));

                // Author
                let author = widget::text::caption("Isa Al-Ula");

                // Version badge
                let version_badge = widget::container(
                    widget::text::caption(env!("CARGO_PKG_VERSION")).width(Length::Shrink),
                )
                .padding([2, space_s])
                .class(cosmic::theme::Container::Primary);

                // Header column (icon + name + author + version)
                let header = widget::column::with_capacity(4)
                    .push(icon)
                    .push(app_name)
                    .push(author)
                    .push(version_badge)
                    .spacing(space_xs)
                    .align_x(cosmic::iced::Alignment::Center)
                    .width(Length::Fill);

                // Links section
                let links_section = widget::settings::section()
                    .title(fl!("about-links"))
                    .add(widget::settings::item(
                        fl!("about-repository"),
                        widget::button::icon(widget::icon::from_name("web-browser-symbolic"))
                            .extra_small()
                            .on_press(Message::OpenUrl(
                                "https://github.com/al-ula/cosmic-cliphoard".into(),
                            ))
                            .class(cosmic::theme::Button::Text),
                    ))
                    .add(widget::settings::item(
                        fl!("about-support"),
                        widget::button::icon(widget::icon::from_name("web-browser-symbolic"))
                            .extra_small()
                            .on_press(Message::OpenUrl(
                                "https://github.com/al-ula/cosmic-cliphoard/issues".into(),
                            ))
                            .class(cosmic::theme::Button::Text),
                    ));

                // Developers section
                let developers_section = widget::settings::section()
                    .title(fl!("about-developers"))
                    .add(widget::settings::item(
                        "Isa Al-Ula",
                        widget::button::icon(widget::icon::from_name("web-browser-symbolic"))
                            .extra_small()
                            .on_press(Message::OpenUrl("https://github.com/al-ula".into()))
                            .class(cosmic::theme::Button::Text),
                    ));

                // License section
                let license_section = widget::settings::section().title(fl!("about-license")).add(
                    widget::settings::item(
                        "MPL-2.0",
                        widget::button::icon(widget::icon::from_name("web-browser-symbolic"))
                            .extra_small()
                            .on_press(Message::OpenUrl(
                                "https://www.mozilla.org/en-US/MPL/2.0/".into(),
                            ))
                            .class(cosmic::theme::Button::Text),
                    ),
                );

                let sections = widget::settings::view_column(vec![
                    header.into(),
                    links_section.into(),
                    developers_section.into(),
                    license_section.into(),
                ]);

                widget::container(widget::scrollable(sections).height(Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(space_s)
                    .into()
            }
        };

        let footer_divider = widget::container(
            widget::container(widget::horizontal_space())
                .height(Length::Fixed(1.0))
                .width(Length::Fill)
                .class(cosmic::theme::Container::Primary),
        )
        .padding(0);

        let main_col = widget::column::with_capacity(3)
            .push(main_row)
            .push(footer_divider)
            .push(footer);

        let panel = widget::container(main_col)
            .padding(space_s)
            .width(Length::Fixed(if self.show_preview {
                LIST_PANEL_WIDTH + PREVIEW_PANEL_WIDTH + f32::from(space_s) * 2.0
            } else {
                LIST_PANEL_WIDTH + f32::from(space_s) * 2.0
            }))
            .height(Length::Fixed(CARD_HEIGHT))
            .class(cosmic::theme::Container::Custom(Box::new(
                |theme: &Theme| {
                    let cosmic = theme.cosmic();
                    cosmic::iced::widget::container::Style {
                        icon_color: Some(Color::from(cosmic.on_bg_color())),
                        text_color: Some(Color::from(cosmic.on_bg_color())),
                        background: Some(cosmic::iced::Background::Color(cosmic.bg_color().into())),
                        shadow: cosmic::iced::Shadow {
                            color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                            offset: cosmic::iced::Vector::new(0.0, 4.0),
                            blur_radius: 20.0,
                        },
                        border: cosmic::iced::Border {
                            radius: cosmic.corner_radii.radius_m.into(),
                            width: 1.0,
                            color: cosmic.palette.neutral_5.into(),
                        },
                    }
                },
            )));

        let panel: Element<'_, Message> = if self.show_tips {
            let tips = self.tips_view();
            cosmic::iced::widget::stack![
                panel,
                widget::container(tips)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .align_y(cosmic::iced::alignment::Vertical::Center)
                    .padding(space_s),
            ]
            .into()
        } else if self.show_daemon_notice {
            let notice = self.daemon_notice_view();
            cosmic::iced::widget::stack![
                panel,
                widget::container(notice)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(cosmic::iced::alignment::Horizontal::Center)
                    .align_y(cosmic::iced::alignment::Vertical::Center)
                    .padding(space_s),
            ]
            .into()
        } else {
            panel.into()
        };

        let panel = widget::mouse_area(panel).on_press(Message::NoOp);

        let scrim = widget::mouse_area(
            widget::container(panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(cosmic::iced::alignment::Horizontal::Center)
                .align_y(cosmic::iced::alignment::Vertical::Center),
        )
        .on_press(Message::Hide);

        scrim.into()
    }

    fn subscription(&self) -> Subscription<Message> {
        struct RefreshSub;

        let mut subs = vec![listen_raw(|event, status, _window| {
            // Check if we're capturing a keybinding
            let capturing = CAPTURING.read().map(|c| *c).unwrap_or(false);

            if capturing {
                if let Event::Keyboard(keyboard::Event::KeyPressed {
                    ref key, modifiers, ..
                }) = event
                {
                    // Escape cancels capture
                    if matches!(key, keyboard::Key::Named(Named::Escape)) {
                        return Some(Message::CancelCaptureKeybinding);
                    }

                    // Build keybinding string from the key event
                    let keybinding_str = key_event_to_keybinding_str(key, &modifiers);
                    if !keybinding_str.is_empty() {
                        return Some(Message::KeybindingCaptured(keybinding_str));
                    }
                }
                return None;
            }

            // Check configurable bindings from the global config
            if let Event::Keyboard(keyboard::Event::KeyPressed {
                ref key, modifiers, ..
            }) = event
            {
                let config = KEYBINDINGS
                    .read()
                    .ok()
                    .and_then(|kb| kb.clone())
                    .unwrap_or_default();

                if status == event::Status::Ignored {
                    if config.toggle_pin.matches(key, modifiers) {
                        return Some(Message::TogglePinSelected);
                    }
                    if config.toggle_preview.matches(key, modifiers) {
                        return Some(Message::TogglePreview);
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

            // Hardcoded navigation bindings
            match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: keyboard::Key::Named(Named::Escape),
                    ..
                }) if status == event::Status::Ignored => Some(Message::Hide),

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

                Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    event::wayland::Event::Layer(event::wayland::LayerEvent::Unfocused, ..),
                )) => Some(Message::LayerUnfocused),
                _ => None,
            }
        })];

        if self.visible {
            subs.push(Subscription::run_with_id(
                std::any::TypeId::of::<RefreshSub>(),
                cosmic::iced::stream::channel(4, move |mut channel| async move {
                    loop {
                        _ = channel.send(Message::RefreshEntries).await;
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }),
            ));
        }

        Subscription::batch(subs)
    }
}

fn key_event_to_keybinding_str(key: &keyboard::Key, modifiers: &Modifiers) -> String {
    let mut parts = Vec::new();

    if modifiers.control() {
        parts.push("ctrl");
    }
    if modifiers.alt() {
        parts.push("alt");
    }
    if modifiers.shift() {
        parts.push("shift");
    }

    let key_str = match key {
        keyboard::Key::Named(named) => match named {
            Named::Delete => "Delete".to_string(),
            Named::ArrowLeft => "ArrowLeft".to_string(),
            Named::ArrowRight => "ArrowRight".to_string(),
            Named::ArrowUp => "ArrowUp".to_string(),
            Named::ArrowDown => "ArrowDown".to_string(),
            Named::Enter => "Enter".to_string(),
            Named::Escape => "Escape".to_string(),
            Named::Backspace => "Backspace".to_string(),
            Named::Tab => "Tab".to_string(),
            Named::Space => "Space".to_string(),
            Named::Home => "Home".to_string(),
            Named::End => "End".to_string(),
            Named::PageUp => "PageUp".to_string(),
            Named::PageDown => "PageDown".to_string(),
            Named::Insert => "Insert".to_string(),
            Named::F1 => "F1".to_string(),
            Named::F2 => "F2".to_string(),
            Named::F3 => "F3".to_string(),
            Named::F4 => "F4".to_string(),
            Named::F5 => "F5".to_string(),
            Named::F6 => "F6".to_string(),
            Named::F7 => "F7".to_string(),
            Named::F8 => "F8".to_string(),
            Named::F9 => "F9".to_string(),
            Named::F10 => "F10".to_string(),
            Named::F11 => "F11".to_string(),
            Named::F12 => "F12".to_string(),
            _ => return String::new(),
        },
        keyboard::Key::Character(c) => c.to_string(),
        _ => return String::new(),
    };

    parts.push(&key_str);
    parts.join("+")
}

async fn fetch_entries() -> Result<Vec<ClipboardEntry>, String> {
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| format!("{e}"))?;

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

    let mut should_exit = false;
    match action {
        DaemonAction::Paste(id) => {
            proxy.paste_entry(id).await.map_err(|e| e.to_string())?;
            should_exit = true;
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
    Ok(should_exit)
}

async fn update_daemon_config(
    max_unpinned: u64,
    max_pinned: u64,
    max_entry_size: u64,
) -> Result<(), String> {
    let conn = zbus::Connection::session()
        .await
        .map_err(|e| format!("{e}"))?;

    let proxy = crate::schema::dbus::ClipboardManagerProxy::builder(&conn)
        .destination(DBUS_NAME)
        .map_err(|e| e.to_string())?
        .path(DBUS_PATH)
        .map_err(|e| e.to_string())?
        .build()
        .await
        .map_err(|e| e.to_string())?;

    proxy
        .update_config(max_unpinned, max_pinned, max_entry_size)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
