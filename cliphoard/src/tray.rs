// SPDX-License-Identifier: MPL-2.0

//! System tray mode using ksni.

use ksni::menu::StandardItem;
use ksni::{Category, MenuItem, Status, ToolTip, TrayService};
use tracing::{debug, error, info};
use zbus::Connection;

const DBUS_NAME: &str = cliphoard_schema::APP_ID;
const DBUS_PATH: &str = cliphoard_schema::DBUS_PATH;

struct TrayIcon;

impl ksni::Tray for TrayIcon {
    fn title(&self) -> String {
        "Cliphoard".into()
    }

    fn icon_name(&self) -> String {
        cliphoard_schema::APP_ID.into()
    }

    fn category(&self) -> Category {
        Category::ApplicationStatus
    }

    fn status(&self) -> Status {
        Status::Active
    }

    fn tool_tip(&self) -> ToolTip {
        ToolTip {
            title: "Cliphoard".into(),
            description: "Clipboard Manager".into(),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            MenuItem::Standard(StandardItem {
                label: "Open".into(),
                icon_name: "window-pop-out-symbolic".into(),
                activate: Box::new(|_this| {
                    debug!("Opening cliphoard overlay");
                    crate::launcher::spawn(std::iter::empty::<&str>()).ok();
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Clear".into(),
                icon_name: "edit-clear".into(),
                activate: Box::new(|_this| {
                    debug!("Clear history requested");
                    clear_history_blocking();
                }),
                ..Default::default()
            }),
            MenuItem::Separator,
            MenuItem::Standard(StandardItem {
                label: "Settings".into(),
                icon_name: "application-menu-symbolic".into(),
                activate: Box::new(|_this| {
                    debug!("Settings requested (not implemented)");
                }),
                ..Default::default()
            }),
        ]
    }
}

fn clear_history_blocking() {
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to create tokio runtime: {}", e);
            return;
        }
    };

    rt.block_on(async {
        if let Err(e) = clear_history().await {
            error!("Failed to clear history: {}", e);
        }
    });
}

async fn clear_history() -> Result<(), String> {
    let conn = Connection::session()
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

    proxy.clear().await.map_err(|e| e.to_string())?;
    info!("Clipboard history cleared");
    Ok(())
}

pub fn run() {
    #[cfg(debug_assertions)]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    #[cfg(not(debug_assertions))]
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting system tray");

    let tray = TrayIcon;
    let service = TrayService::new(tray);

    info!("Tray service created");

    service.spawn();

    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
