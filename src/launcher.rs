// SPDX-License-Identifier: MPL-2.0

#[cfg(not(any(feature = "launcher-desktop")))]
pub fn spawn<I, S>(args: I) -> std::io::Result<std::process::Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let bin = std::env::current_exe().unwrap_or_else(|_| {
        const FALLBACK: &str = match option_env!("CLIPHOARD_BIN") {
            Some(v) => v,
            None => "cliphoard",
        };
        std::path::PathBuf::from(FALLBACK)
    });
    std::process::Command::new(bin).args(args).spawn()
}

#[cfg(feature = "launcher-desktop")]
pub fn spawn<I, S>(args: I) -> std::io::Result<std::process::Child>
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    std::process::Command::new("gio")
        .args(["launch", &format!("{}.desktop", crate::schema::APP_ID)])
        .args(args)
        .spawn()
}
