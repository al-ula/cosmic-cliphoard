// SPDX-License-Identifier: MPL-2.0

use cosmic::cctk::{
    sctk::reexports::protocols_wlr::data_control::v1::client::{
        zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
        zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
        zwlr_data_control_offer_v1::ZwlrDataControlOfferV1,
        zwlr_data_control_source_v1::{self, ZwlrDataControlSourceV1},
    },
    wayland_client::{
        Connection, Dispatch, Proxy, event_created_child,
        globals::{registry_queue_init, GlobalListContents},
        protocol::{
            wl_registry::WlRegistry,
            wl_seat::WlSeat,
        },
    },
};
use std::fs::File;
use std::io::Write;

#[derive(thiserror::Error, Debug)]
pub enum ClipboardWriteError {
    #[error("Failed to connect to Wayland compositor")]
    Connect(#[from] cosmic::cctk::wayland_client::ConnectError),

    #[error("Wayland dispatch error")]
    Dispatch(#[from] cosmic::cctk::wayland_client::DispatchError),

    #[error("Missing required protocol: {name} v{version}")]
    MissingProtocol {
        name: &'static str,
        version: u32,
    },

    #[error("No seats available")]
    NoSeats,

    #[error("Clipboard setup failed: {0}")]
    Setup(String),
}

struct WriterState {
    done: bool,
    data: Vec<u8>,
}

impl Dispatch<WlRegistry, GlobalListContents> for WriterState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        _event: <WlRegistry as Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qh: &cosmic::cctk::wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSeat, ()> for WriterState {
    fn event(
        _state: &mut Self,
        _seat: &WlSeat,
        _event: <WlSeat as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &cosmic::cctk::wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlManagerV1, ()> for WriterState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrDataControlManagerV1,
        _event: <ZwlrDataControlManagerV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &cosmic::cctk::wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for WriterState {
    fn event(
        _state: &mut Self,
        _device: &ZwlrDataControlDeviceV1,
        _event: <ZwlrDataControlDeviceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &cosmic::cctk::wayland_client::QueueHandle<Self>,
    ) {
    }

    event_created_child!(WriterState, ZwlrDataControlDeviceV1, [
        zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, ()),
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for WriterState {
    fn event(
        _state: &mut Self,
        _offer: &ZwlrDataControlOfferV1,
        _event: <ZwlrDataControlOfferV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &cosmic::cctk::wayland_client::QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlSourceV1, ()> for WriterState {
    fn event(
        state: &mut Self,
        _source: &ZwlrDataControlSourceV1,
        event: <ZwlrDataControlSourceV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &cosmic::cctk::wayland_client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_source_v1::Event::Send { fd, .. } => {
                let mut file = File::from(fd);
                let _ = file.write_all(&state.data);
            }
            zwlr_data_control_source_v1::Event::Cancelled => {
                state.done = true;
            }
            _ => {}
        }
    }
}

/// Write data to the Wayland clipboard using the data control protocol.
///
/// The `ready_tx` channel is signalled after the roundtrip confirms the compositor
/// accepted the selection. The function then continues to block, serving `Send`
/// events until the source is `Cancelled` (replaced by another copy).
pub fn write_to_clipboard(
    mime_type: String,
    data: Vec<u8>,
    ready_tx: tokio::sync::oneshot::Sender<Result<(), ClipboardWriteError>>,
) -> Result<(), ClipboardWriteError> {
    let setup = || -> Result<(cosmic::cctk::wayland_client::EventQueue<WriterState>, WriterState), ClipboardWriteError> {
        let conn = Connection::connect_to_env()?;

        let (globals, mut queue) = registry_queue_init::<WriterState>(&conn).map_err(|err| {
            match err {
                cosmic::cctk::wayland_client::globals::GlobalError::Backend(err) => {
                    ClipboardWriteError::Dispatch(err.into())
                }
                cosmic::cctk::wayland_client::globals::GlobalError::InvalidId(_) => {
                    panic!("Invalid wl_registry ID")
                }
            }
        })?;

        let qh = queue.handle();

        let manager: ZwlrDataControlManagerV1 = match globals.bind(&qh, 1..=1, ()) {
            Ok(m) => m,
            Err(_) => {
                return Err(ClipboardWriteError::MissingProtocol {
                    name: ZwlrDataControlManagerV1::interface().name,
                    version: 1,
                });
            }
        };

        let registry = globals.registry();
        let seats: Vec<WlSeat> = globals.contents().with_list(|list| {
            list.iter()
                .filter(|g| g.interface == WlSeat::interface().name && g.version >= 2)
                .map(|g| registry.bind(g.name, 2, &qh, ()))
                .collect()
        });

        if seats.is_empty() {
            return Err(ClipboardWriteError::NoSeats);
        }

        let seat = seats.into_iter().next().unwrap();
        let device = manager.get_data_device(&seat, &qh, ());
        let source = manager.create_data_source(&qh, ());
        source.offer(mime_type.clone());
        // Offer common aliases so that consumers requesting a slightly
        // different mime string can still match our source.
        if mime_type == "text/plain" {
            source.offer("text/plain;charset=utf-8".to_string());
            source.offer("TEXT".to_string());
            source.offer("STRING".to_string());
            source.offer("UTF8_STRING".to_string());
        } else if mime_type == "text/plain;charset=utf-8" {
            source.offer("text/plain".to_string());
            source.offer("TEXT".to_string());
            source.offer("STRING".to_string());
            source.offer("UTF8_STRING".to_string());
        }
        device.set_selection(Some(&source));

        let mut state = WriterState {
            done: false,
            data,
        };

        queue.roundtrip(&mut state)?;

        Ok((queue, state))
    };

    match setup() {
        Ok((mut queue, mut state)) => {
            // Signal that the clipboard is set
            let _ = ready_tx.send(Ok(()));

            // Block serving Send events until Cancelled
            while !state.done {
                if let Err(e) = queue.blocking_dispatch(&mut state) {
                    tracing::error!("Clipboard dispatch error: {}", e);
                    break;
                }
            }
            Ok(())
        }
        Err(e) => {
            // Send the error description through the channel since the
            // error type itself isn't Clone
            let desc = e.to_string();
            let _ = ready_tx.send(Err(ClipboardWriteError::Setup(desc)));
            Err(e)
        }
    }
}
