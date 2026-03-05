// SPDX-License-Identifier: MPL-2.0

mod app;
mod applet;
mod cli;
mod commands;
mod config;
mod daemon;
mod decode;
mod i18n;
mod launcher;
mod schema;
mod tray;

use clap::Parser;
use tracing::error;

fn main() -> cosmic::iced::Result {
    let cli = cli::Cli::parse();

    match cli.command {
        None => {
            let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
            i18n::init(&requested_languages);

            app::run_overlay()
        }
        Some(cli::Command::Settings) => {
            let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
            i18n::init(&requested_languages);

            app::run_overlay_with_view(app::View::Settings)
        }
        Some(cli::Command::Applet) => {
            let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
            i18n::init(&requested_languages);

            cosmic::applet::run::<applet::AppletModel>(())
        }
        Some(cli::Command::Tray) => {
            tray::run();
            Ok(())
        }
        Some(cli::Command::Daemon) => {
            if let Err(e) = tokio::runtime::Runtime::new()
                .expect("failed to create tokio runtime")
                .block_on(commands::run_daemon())
            {
                error!("daemon error: {e}");
                let mut source = std::error::Error::source(&*e);
                while let Some(s) = source {
                    error!("  caused by: {s}");
                    source = std::error::Error::source(s);
                }
                std::process::exit(1);
            }
            Ok(())
        }
        Some(cli::Command::Decode { mime, json }) => {
            commands::run_decode(mime, json);
            Ok(())
        }
        Some(cli::Command::GenerateService { path }) => {
            commands::generate_service(path);
            Ok(())
        }
        Some(cmd) => {
            tokio::runtime::Runtime::new()
                .expect("failed to create tokio runtime")
                .block_on(commands::run(cmd));
            Ok(())
        }
    }
}
