//! Real `x11rb`-backed `X11Connection`. Only exercised by the `#[ignore]`d
//! integration tests in this module (see their doc comments) since it needs
//! a live X server; the unit-tested logic in `x11/mod.rs` runs entirely
//! against the in-memory fake instead.
//!
//! Run the integration tests manually against a real or `Xvfb`-hosted X11
//! session with:
//! ```sh
//! Xvfb :99 &
//! DISPLAY=:99 cargo test -p clip-platform -- --ignored
//! ```

use super::{WindowId, X11Connection};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use x11rb::connection::Connection as _;
use x11rb::protocol::xfixes;
use x11rb::protocol::xproto::{self, Atom, AtomEnum, EventMask, PropMode, Window, WindowClass};
use x11rb::protocol::xtest;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;
use x11rb::COPY_DEPTH_FROM_PARENT;

struct Atoms {
    clipboard: Atom,
    utf8_string: Atom,
    targets: Atom,
    wm_class: Atom,
    net_wm_name: Atom,
    net_active_window: Atom,
}

impl Atoms {
    fn intern(conn: &RustConnection) -> Result<Self, Box<dyn std::error::Error>> {
        let clipboard = xproto::intern_atom(conn, false, b"CLIPBOARD")?.reply()?.atom;
        let utf8_string = xproto::intern_atom(conn, false, b"UTF8_STRING")?.reply()?.atom;
        let targets = xproto::intern_atom(conn, false, b"TARGETS")?.reply()?.atom;
        let wm_class = xproto::intern_atom(conn, false, b"WM_CLASS")?.reply()?.atom;
        let net_wm_name = xproto::intern_atom(conn, false, b"_NET_WM_NAME")?.reply()?.atom;
        let net_active_window = xproto::intern_atom(conn, false, b"_NET_ACTIVE_WINDOW")?.reply()?.atom;
        Ok(Self { clipboard, utf8_string, targets, wm_class, net_wm_name, net_active_window })
    }
}

/// Content this process is currently offering as the `CLIPBOARD` selection
/// owner, served to other clients' `SelectionRequest`s by the background
/// event-pump thread. `write_selection`/`write_selection_target` each clear
/// the other field, since a given paste only ever offers one or the other.
#[derive(Default)]
struct OwnedContent {
    text: Option<String>,
    image: Option<(Atom, Vec<u8>)>,
}

/// Real X11 connection: owns a hidden helper window used both as the
/// selection requestor/owner and as the `XTestFakeInput` focus target, plus a
/// background thread pumping the X event queue (serving `SelectionRequest`s
/// for content we own, and forwarding `XFixes` selection-change
/// notifications into `pending_changes`).
pub struct RealX11Connection {
    conn: Arc<RustConnection>,
    helper_window: Window,
    atoms: Atoms,
    owned_content: Arc<Mutex<OwnedContent>>,
    selection_changes: Mutex<Receiver<()>>,
    /// Set by `read_selection` before issuing `ConvertSelection`, so the
    /// background event pump (the sole caller of `wait_for_event`) can
    /// deliver the matching `SelectionNotify` back to the waiting caller
    /// instead of racing it for the same event.
    pending_notify: Arc<Mutex<Option<mpsc::Sender<xproto::SelectionNotifyEvent>>>>,
}

impl RealX11Connection {
    /// Connects to the X server named by `$DISPLAY` (or `dpy_name` if given)
    /// and prepares the helper window + `XFixes` selection-change watch.
    ///
    /// `RustConnection` guards its internal state behind mutexes and is
    /// `Send + Sync`, so the single connection is shared (via `Arc`) between
    /// this handle and the background event-pump thread below, rather than
    /// opening a second connection - a `SelectionRequest` is delivered by the
    /// server only to the specific client connection that owns the
    /// selection, so serving it must happen on the owning connection.
    pub fn connect(dpy_name: Option<&str>) -> Result<Self, Box<dyn std::error::Error>> {
        let (conn, screen_num) = RustConnection::connect(dpy_name)?;
        let conn = Arc::new(conn);
        let root = conn.setup().roots[screen_num].root;
        let atoms = Atoms::intern(&conn)?;

        let helper_window = conn.generate_id()?;
        xproto::create_window(
            &conn,
            COPY_DEPTH_FROM_PARENT,
            helper_window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_ONLY,
            0,
            &xproto::CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )?
        .check()?;

        xfixes::query_version(&conn, 4, 0)?.reply()?;
        xfixes::select_selection_input(
            &conn,
            helper_window,
            atoms.clipboard,
            xfixes::SelectionEventMask::SET_SELECTION_OWNER,
        )?
        .check()?;
        conn.flush()?;

        let (tx, rx) = mpsc::channel::<()>();
        let owned_content = Arc::new(Mutex::new(OwnedContent::default()));
        let pending_notify: Arc<Mutex<Option<mpsc::Sender<xproto::SelectionNotifyEvent>>>> =
            Arc::new(Mutex::new(None));

        let pump_conn = conn.clone();
        let pump_owned_content = owned_content.clone();
        let pump_pending_notify = pending_notify.clone();
        let pump_helper_window = helper_window;
        let pump_atoms_clipboard = atoms.clipboard;
        let pump_atoms_utf8 = atoms.utf8_string;
        let pump_atoms_targets = atoms.targets;

        // Background event pump: the sole caller of `wait_for_event` for this
        // connection. Forwards XFixes selection-change notifications,
        // answers SelectionRequests for content we own, and hands our own
        // pending SelectionNotify (from `read_selection`'s ConvertSelection)
        // back to whichever call is waiting on it. Errors mean the
        // connection died; the thread simply exits.
        std::thread::spawn(move || {
            while let Ok(event) = pump_conn.wait_for_event() {
                match event {
                    Event::XfixesSelectionNotify(notify) if notify.selection == pump_atoms_clipboard => {
                        let _ = tx.send(());
                    }
                    Event::SelectionRequest(request) if request.selection == pump_atoms_clipboard => {
                        let owned = pump_owned_content.lock().unwrap();
                        let text = owned.text.clone();
                        let image = owned.image.clone();
                        drop(owned);
                        let _ = respond_to_selection_request(
                            &pump_conn,
                            &request,
                            text.as_deref(),
                            image.as_ref().map(|(atom, bytes)| (*atom, bytes.as_slice())),
                            pump_atoms_utf8,
                            pump_atoms_targets,
                        );
                    }
                    Event::SelectionNotify(notify) if notify.requestor == pump_helper_window => {
                        if let Some(sender) = pump_pending_notify.lock().unwrap().take() {
                            let _ = sender.send(notify);
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(Self { conn, helper_window, atoms, owned_content, selection_changes: Mutex::new(rx), pending_notify })
    }

    fn get_property_string(&self, window: Window, property: Atom, type_: Atom) -> Option<String> {
        let reply = xproto::get_property(&self.conn, false, window, property, type_, 0, u32::MAX)
            .ok()?
            .reply()
            .ok()?;
        if reply.value.is_empty() {
            return None;
        }
        Some(String::from_utf8_lossy(&reply.value).into_owned())
    }

    /// Claims `CLIPBOARD` ownership so the background event pump starts
    /// receiving `SelectionRequest`s for whatever was just written into
    /// `owned_content`.
    fn claim_selection_ownership(&self) {
        if let Ok(cookie) =
            xproto::set_selection_owner(&self.conn, self.helper_window, self.atoms.clipboard, x11rb::CURRENT_TIME)
        {
            let _ = cookie.check();
        }
        let _ = self.conn.flush();
    }
}

fn respond_to_selection_request(
    conn: &RustConnection,
    request: &xproto::SelectionRequestEvent,
    text: Option<&str>,
    image: Option<(Atom, &[u8])>,
    utf8_string: Atom,
    targets: Atom,
) -> Result<(), Box<dyn std::error::Error>> {
    let property = if request.property == 0 { request.target } else { request.property };
    let image_atom = image.map(|(atom, _)| atom);

    if request.target == targets {
        let mut available = vec![utf8_string, targets];
        available.extend(image_atom);
        let data: Vec<u8> = available.iter().flat_map(|a| a.to_ne_bytes()).collect();
        xproto::change_property(
            conn,
            PropMode::REPLACE,
            request.requestor,
            property,
            AtomEnum::ATOM,
            32,
            available.len() as u32,
            &data,
        )?
        .check()?;
    } else if request.target == utf8_string {
        let bytes = text.unwrap_or("").as_bytes();
        xproto::change_property(
            conn,
            PropMode::REPLACE,
            request.requestor,
            property,
            utf8_string,
            8,
            bytes.len() as u32,
            bytes,
        )?
        .check()?;
    } else if Some(request.target) == image_atom {
        let bytes = image.map(|(_, bytes)| bytes).unwrap_or(&[]);
        xproto::change_property(
            conn,
            PropMode::REPLACE,
            request.requestor,
            property,
            request.target,
            8,
            bytes.len() as u32,
            bytes,
        )?
        .check()?;
    }

    let notify = xproto::SelectionNotifyEvent {
        response_type: xproto::SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: request.time,
        requestor: request.requestor,
        selection: request.selection,
        target: request.target,
        property,
    };
    xproto::send_event(conn, false, request.requestor, EventMask::NO_EVENT, notify)?.check()?;
    conn.flush()?;
    Ok(())
}

impl X11Connection for RealX11Connection {
    fn read_selection(&self) -> Option<String> {
        let owner = xproto::get_selection_owner(&self.conn, self.atoms.clipboard).ok()?.reply().ok()?;
        if owner.owner == x11rb::NONE {
            return None;
        }

        let property = self.atoms.utf8_string;
        let (tx, rx) = mpsc::channel();
        *self.pending_notify.lock().unwrap() = Some(tx);

        xproto::convert_selection(
            &self.conn,
            self.helper_window,
            self.atoms.clipboard,
            self.atoms.utf8_string,
            property,
            x11rb::CURRENT_TIME,
        )
        .ok()?
        .check()
        .ok()?;
        self.conn.flush().ok()?;

        // The background event pump delivers our SelectionNotify here once
        // it arrives; give up after a short timeout rather than hanging
        // forever if the owner never responds.
        let notify = rx.recv_timeout(std::time::Duration::from_millis(500)).ok()?;
        if notify.property == 0 {
            return None;
        }
        self.get_property_string(self.helper_window, property, self.atoms.utf8_string)
    }

    fn read_selection_target(&self, mime: &str) -> Option<Vec<u8>> {
        let owner = xproto::get_selection_owner(&self.conn, self.atoms.clipboard).ok()?.reply().ok()?;
        if owner.owner == x11rb::NONE {
            return None;
        }

        let target = xproto::intern_atom(&self.conn, false, mime.as_bytes()).ok()?.reply().ok()?.atom;
        let property = target;
        let (tx, rx) = mpsc::channel();
        *self.pending_notify.lock().unwrap() = Some(tx);

        xproto::convert_selection(
            &self.conn,
            self.helper_window,
            self.atoms.clipboard,
            target,
            property,
            x11rb::CURRENT_TIME,
        )
        .ok()?
        .check()
        .ok()?;
        self.conn.flush().ok()?;

        let notify = rx.recv_timeout(std::time::Duration::from_millis(500)).ok()?;
        if notify.property == 0 {
            return None;
        }
        let reply = xproto::get_property(&self.conn, false, self.helper_window, property, AtomEnum::ANY, 0, u32::MAX)
            .ok()?
            .reply()
            .ok()?;
        if reply.value.is_empty() {
            None
        } else {
            Some(reply.value)
        }
    }

    fn write_selection(&self, content: &str) {
        let mut owned = self.owned_content.lock().unwrap();
        owned.text = Some(content.to_string());
        owned.image = None;
        drop(owned);
        self.claim_selection_ownership();
    }

    fn write_selection_target(&self, mime: &str, bytes: &[u8]) {
        let Ok(cookie) = xproto::intern_atom(&self.conn, false, mime.as_bytes()) else { return };
        let Ok(atom) = cookie.reply() else { return };
        let mut owned = self.owned_content.lock().unwrap();
        owned.text = None;
        owned.image = Some((atom.atom, bytes.to_vec()));
        drop(owned);
        self.claim_selection_ownership();
    }

    fn poll_selection_change(&self) -> Option<()> {
        self.selection_changes.lock().unwrap().recv().ok()
    }

    fn window_property(&self, window: WindowId, name: &str) -> Option<String> {
        let atom = match name {
            "WM_CLASS" => self.atoms.wm_class,
            "_NET_WM_NAME" => self.atoms.net_wm_name,
            _ => xproto::intern_atom(&self.conn, true, name.as_bytes()).ok()?.reply().ok()?.atom,
        };
        self.get_property_string(window as Window, atom, AtomEnum::ANY.into())
    }

    fn focused_window(&self) -> Option<WindowId> {
        let root = self.conn.setup().roots.first()?.root;
        let reply = xproto::get_property(
            &self.conn,
            false,
            root,
            self.atoms.net_active_window,
            AtomEnum::WINDOW,
            0,
            1,
        )
        .ok()?
        .reply()
        .ok()?;
        if reply.value.len() < 4 {
            return None;
        }
        let window = u32::from_ne_bytes(reply.value[0..4].try_into().ok()?);
        if window == 0 {
            None
        } else {
            Some(window)
        }
    }

    fn synthesize_key(&self, window: WindowId, binding: &str) -> Result<(), String> {
        let keycodes = binding_to_keycodes(&self.conn, binding).map_err(|e| e.to_string())?;

        xproto::set_input_focus(&self.conn, xproto::InputFocus::PARENT, window as Window, x11rb::CURRENT_TIME)
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| e.to_string())?;

        let root = self.conn.setup().roots.first().map(|s| s.root).unwrap_or(0);
        for &keycode in &keycodes {
            xtest::fake_input(&self.conn, xproto::KEY_PRESS_EVENT, keycode, 0, root, 0, 0, 0)
                .map_err(|e| e.to_string())?
                .check()
                .map_err(|e| e.to_string())?;
        }
        for &keycode in keycodes.iter().rev() {
            xtest::fake_input(&self.conn, xproto::KEY_RELEASE_EVENT, keycode, 0, root, 0, 0, 0)
                .map_err(|e| e.to_string())?
                .check()
                .map_err(|e| e.to_string())?;
        }
        self.conn.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Parses a binding like `"ctrl+shift+v"` into X11 keycodes (modifiers first,
/// then the base key), looking each keysym up via `GetKeyboardMapping`.
fn binding_to_keycodes(conn: &RustConnection, binding: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let setup = conn.setup();
    let min_keycode = setup.min_keycode;
    let count = setup.max_keycode - setup.min_keycode + 1;
    let mapping = xproto::get_keyboard_mapping(conn, min_keycode, count)?.reply()?;
    let per_keycode = mapping.keysyms_per_keycode as usize;

    let keysym_to_keycode = |keysym: u32| -> Option<u8> {
        mapping
            .keysyms
            .chunks(per_keycode.max(1))
            .position(|chunk| chunk.contains(&keysym))
            .map(|index| min_keycode + index as u8)
    };

    let mut keycodes = Vec::new();
    for part in binding.split('+') {
        let keysym = match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => 0xffe3, // Control_L
            "shift" => 0xffe1,            // Shift_L
            "alt" => 0xffe9,              // Alt_L
            "super" | "meta" => 0xffeb,   // Super_L
            other if other.len() == 1 => other.chars().next().unwrap() as u32,
            other => return Err(format!("unrecognized key token '{other}'").into()),
        };
        let keycode = keysym_to_keycode(keysym)
            .ok_or_else(|| format!("no keycode mapped for keysym {keysym:#x}"))?;
        keycodes.push(keycode);
    }
    Ok(keycodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::x11::X11Backend;

    /// Exercises the real `x11rb`-backed connection end-to-end: write to the
    /// clipboard, read it back, and confirm the round trip. Needs a live X
    /// server (see this module's top-level doc comment for how to run it
    /// under `Xvfb`).
    #[test]
    #[ignore = "requires a live X11 server; run manually, e.g. under Xvfb"]
    fn real_connection_round_trips_clipboard_content() {
        let conn = RealX11Connection::connect(None).expect("connect to X server");
        let blob_dir = std::env::temp_dir().join("clipdeck-integration-test-blobs");
        let backend = X11Backend::new(conn, blob_dir);
        backend.set_current("clipdeck integration test").unwrap();
        let snapshot = backend.read_current().unwrap();
        assert_eq!(
            snapshot.representations.first().and_then(|r| r.text_value.as_deref()),
            Some("clipdeck integration test")
        );
    }
}
