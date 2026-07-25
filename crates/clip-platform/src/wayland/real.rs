//! Real `wayland-client`-backed `WaylandConnection`, using the
//! `wlr-data-control` protocol. Only exercised by the `#[ignore]`d
//! integration test in this module (see its doc comment) since it needs a
//! live Wayland compositor supporting the protocol; the unit-tested logic in
//! `wayland/mod.rs` runs entirely against the in-memory fake instead.
//!
//! Run the integration test manually against a compositor that supports
//! `wlr-data-control` (e.g. `sway`), with `WAYLAND_DISPLAY` set:
//! ```sh
//! cargo test -p clip-platform -- --ignored wayland::real
//! ```

use std::io::{Read, Write};
use std::os::fd::{AsFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{wl_registry, wl_seat::WlSeat};
use wayland_client::{delegate_noop, Connection, Dispatch, QueueHandle};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
    zwlr_data_control_source_v1::{self, ZwlrDataControlSourceV1},
};

use super::WaylandConnection;

/// MIME types tried, in preference order, when reading a text selection.
const TEXT_MIME_TYPES: &[&str] = &["text/plain;charset=utf-8", "text/plain", "UTF8_STRING", "STRING"];

#[derive(Default)]
struct OfferState {
    offer: Option<ZwlrDataControlOfferV1>,
    mime_types: Vec<String>,
}

/// State shared between the connection handle (used by `read_selection`/
/// `write_selection`) and the background thread driving the event queue.
#[derive(Default)]
struct Shared {
    owned_text: Mutex<Option<String>>,
    current_offer: Mutex<OfferState>,
}

struct AppState {
    shared: Arc<Shared>,
    selection_tx: Sender<()>,
}

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for AppState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        // Dynamic global add/remove after startup isn't acted on.
    }
}

delegate_noop!(AppState: ignore WlSeat);
delegate_noop!(AppState: ignore ZwlrDataControlManagerV1);

impl Dispatch<ZwlrDataControlSourceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlSourceV1,
        event: zwlr_data_control_source_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_data_control_source_v1::Event::Send { mime_type: _, fd } = event {
            if let Some(text) = state.shared.owned_text.lock().unwrap().clone() {
                let mut stream = UnixStream::from(fd);
                let _ = stream.write_all(text.as_bytes());
            }
        }
    }
}

impl Dispatch<ZwlrDataControlOfferV1, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrDataControlOfferV1,
        event: zwlr_data_control_offer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_data_control_offer_v1::Event::Offer { mime_type } = event {
            let mut current = state.shared.current_offer.lock().unwrap();
            if current.offer.as_ref() == Some(proxy) {
                current.mime_types.push(mime_type);
            }
        }
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrDataControlDeviceV1,
        event: zwlr_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_data_control_device_v1::Event::DataOffer { id } => {
                *state.shared.current_offer.lock().unwrap() = OfferState { offer: Some(id), mime_types: Vec::new() };
            }
            zwlr_data_control_device_v1::Event::Selection { id: Some(_) } => {
                let _ = state.selection_tx.send(());
            }
            _ => {}
        }
    }
}

/// Real Wayland connection: binds `wl_seat` and (if advertised)
/// `zwlr_data_control_manager_v1`, then a data-control device for that seat,
/// plus a background thread driving the event queue (delivering
/// `data_offer`/`selection` notifications and serving `send` requests for
/// content we own).
pub struct RealWaylandConnection {
    conn: Connection,
    qh: QueueHandle<AppState>,
    manager: Option<ZwlrDataControlManagerV1>,
    device: Option<ZwlrDataControlDeviceV1>,
    shared: Arc<Shared>,
    selection_changes: Mutex<std::sync::mpsc::Receiver<()>>,
    data_control_supported: bool,
}

impl RealWaylandConnection {
    pub fn connect() -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::connect_to_env()?;
        let (globals, event_queue) = registry_queue_init::<AppState>(&conn)?;
        let qh = event_queue.handle();

        let seat: WlSeat = globals.bind(&qh, 1..=1, ())?;
        let manager_bind: Result<ZwlrDataControlManagerV1, _> = globals.bind(&qh, 1..=2, ());

        let shared = Arc::new(Shared::default());
        let (tx, rx) = mpsc::channel();

        let (manager, device, data_control_supported) = match manager_bind {
            Ok(manager) => {
                let device = manager.get_data_device(&seat, &qh, ());
                (Some(manager), Some(device), true)
            }
            Err(_) => (None, None, false),
        };

        let mut app_state = AppState { shared: shared.clone(), selection_tx: tx };
        let mut queue_for_thread = event_queue;
        std::thread::spawn(move || {
            while queue_for_thread.blocking_dispatch(&mut app_state).is_ok() {}
        });

        Ok(Self {
            conn,
            qh,
            manager,
            device,
            shared,
            selection_changes: Mutex::new(rx),
            data_control_supported,
        })
    }
}

impl WaylandConnection for RealWaylandConnection {
    fn supports_data_control(&self) -> bool {
        self.data_control_supported
    }

    fn read_selection(&self) -> Option<String> {
        let (offer, mime) = {
            let state = self.shared.current_offer.lock().unwrap();
            let offer = state.offer.clone()?;
            let mime = TEXT_MIME_TYPES.iter().find(|candidate| state.mime_types.iter().any(|m| m == *candidate))?;
            (offer, mime.to_string())
        };

        let (write_end, mut read_end) = UnixStream::pair().ok()?;
        let write_fd: OwnedFd = write_end.into();
        offer.receive(mime, write_fd.as_fd());
        self.conn.flush().ok()?;
        drop(write_fd);

        let mut buf = Vec::new();
        read_end.read_to_end(&mut buf).ok()?;
        if buf.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(&buf).into_owned())
        }
    }

    fn write_selection(&self, content: &str) {
        let (Some(manager), Some(device)) = (&self.manager, &self.device) else { return };
        *self.shared.owned_text.lock().unwrap() = Some(content.to_string());
        let source = manager.create_data_source(&self.qh, ());
        source.offer("text/plain;charset=utf-8".to_string());
        source.offer("text/plain".to_string());
        device.set_selection(Some(&source));
        let _ = self.conn.flush();
    }

    fn poll_selection_change(&self) -> Option<()> {
        self.selection_changes.lock().unwrap().recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wayland::WaylandBackend;

    /// Exercises the real `wayland-client`-backed connection end-to-end:
    /// write to the clipboard, read it back, and confirm the round trip.
    /// Needs a live Wayland compositor with `wlr-data-control` support (see
    /// this module's top-level doc comment).
    #[test]
    #[ignore = "requires a live Wayland compositor with wlr-data-control support"]
    fn real_connection_round_trips_clipboard_content() {
        let conn = RealWaylandConnection::connect().expect("connect to Wayland compositor");
        let backend = WaylandBackend::new(conn).expect("compositor supports wlr-data-control");
        backend.set_current("clipdeck integration test").unwrap();
        // Give the background thread a moment to process our own `selection`
        // notification before reading it back.
        std::thread::sleep(std::time::Duration::from_millis(200));
        let snapshot = backend.read_current().unwrap();
        assert_eq!(
            snapshot.representations.first().and_then(|r| r.text_value.as_deref()),
            Some("clipdeck integration test")
        );
    }
}
