// SPDX-License-Identifier: MPL-2.0

/// Spawn a new instance of the application with the given arguments.
///
/// The launch strategy is selected at compile time via feature flags:
/// - (default) direct binary spawn
/// - `launcher-desktop` — gio desktop file activation
/// - `launcher-flatpak` — flatpak run
///
/// The binary name for the default case can be overridden at build time by
/// setting the `CLIPHOARD_BIN` environment variable.

#[cfg(not(any(feature = "launcher-desktop", feature = "launcher-flatpak")))]
pub fn spawn<I, S>(args: I) -> std::io::Result<std::process::Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    const BIN: &str = match option_env!("CLIPHOARD_BIN") {
        Some(v) => v,
        None => "cliphoard",
    };
    std::process::Command::new(BIN).args(args).spawn()
}

#[cfg(feature = "launcher-desktop")]
pub fn spawn<I, S>(args: I) -> std::io::Result<std::process::Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    std::process::Command::new("gio")
        .args(["launch", &format!("{}.desktop", cliphoard_schema::APP_ID)])
        .args(args)
        .spawn()
}

#[cfg(feature = "launcher-flatpak")]
pub fn spawn<I, S>(args: I) -> std::io::Result<std::process::Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    std::process::Command::new("flatpak")
        .args(["run", cliphoard_schema::APP_ID])
        .args(args)
        .spawn()
}
