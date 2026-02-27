// SPDX-License-Identifier: MPL-2.0

#[zbus::proxy(
    interface = "com.github.al_ula.Cliphoard.Manager",
    default_service = "com.github.al_ula.Cliphoard",
    default_path = "/com/github/al_ula/Cliphoard"
)]
pub trait ClipboardManager {
    fn list_entries(&self) -> zbus::fdo::Result<Vec<u8>>;

    fn get_entry(&self, id: u64) -> zbus::fdo::Result<Vec<u8>>;

    fn search(&self, query: &str) -> zbus::fdo::Result<Vec<u8>>;

    fn delete_entry(&self, id: u64) -> zbus::fdo::Result<bool>;

    fn pin_entry(&self, id: u64) -> zbus::fdo::Result<bool>;

    fn unpin_entry(&self, id: u64) -> zbus::fdo::Result<bool>;

    fn clear(&self) -> zbus::fdo::Result<()>;

    fn paste_entry(&self, id: u64) -> zbus::fdo::Result<bool>;

    fn update_config(
        &self,
        max_unpinned: u64,
        max_pinned: u64,
        max_entry_size: u64,
    ) -> zbus::fdo::Result<()>;

    #[zbus(signal)]
    fn entry_added(&self, id: u64) -> zbus::Result<()>;

    #[zbus(signal)]
    fn entry_removed(&self, id: u64) -> zbus::Result<()>;

    #[zbus(signal)]
    fn history_cleared(&self) -> zbus::Result<()>;
}
