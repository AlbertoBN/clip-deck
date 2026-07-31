//! Global hotkey registration.

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

/// A parsed, platform-agnostic hotkey binding (modifiers + one key).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HotkeyBinding {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub super_key: bool,
    pub key: char,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HotkeyError {
    #[error("invalid hotkey binding: '{0}'")]
    InvalidBinding(String),
    #[error("hotkey '{0:?}' is already registered")]
    AlreadyRegistered(HotkeyBinding),
    #[error("hotkey registration is not supported on this compositor")]
    Unsupported,
    #[error("gsettings command failed: {0}")]
    GSettingsFailure(String),
}

/// Parses a binding string like `"Ctrl+Shift+V"` into modifiers + key.
/// Modifiers must come before the single trailing key token.
pub fn parse_binding(spec: &str) -> Result<HotkeyBinding, HotkeyError> {
    let mut binding = HotkeyBinding { ctrl: false, shift: false, alt: false, super_key: false, key: '\0' };
    let mut key_set = false;

    for token in spec.split('+') {
        match token {
            "Ctrl" | "Control" => binding.ctrl = true,
            "Shift" => binding.shift = true,
            "Alt" => binding.alt = true,
            "Super" | "Meta" | "Cmd" => binding.super_key = true,
            other => {
                let mut chars = other.chars();
                let single = chars
                    .next()
                    .filter(|c| (c.is_ascii_alphanumeric() || is_supported_punctuation(*c)) && chars.next().is_none());
                match single {
                    Some(c) if !key_set => {
                        binding.key = c.to_ascii_uppercase();
                        key_set = true;
                    }
                    _ => return Err(HotkeyError::InvalidBinding(spec.to_string())),
                }
            }
        }
    }

    if !key_set {
        return Err(HotkeyError::InvalidBinding(spec.to_string()));
    }
    Ok(binding)
}

/// Punctuation keys with a corresponding `global_hotkey::hotkey::Code`
/// variant (mirrors the set handled by `char_to_code`).
fn is_supported_punctuation(c: char) -> bool {
    matches!(c, '`' | '-' | '=' | '[' | ']' | '\\' | ',' | '.' | '\'' | ';' | '/')
}

/// Registration backend seam: implemented once for real via the
/// `global-hotkey` crate, and once as an in-memory fake for unit tests.
pub trait HotkeyBackend: Send + Sync {
    fn register(
        &self,
        binding: HotkeyBinding,
        callback: Box<dyn Fn() + Send + Sync>,
    ) -> Result<(), HotkeyError>;

    /// Whether this backend has a usable global-shortcut mechanism at all.
    /// Defaults to `true`; a compositor/session with no such mechanism (e.g.
    /// Wayland without portal-based global shortcuts, see
    /// `UnsupportedHotkeyBackend`) overrides this to `false`, so callers can
    /// report it via `BackendCapabilities` without needing to attempt a
    /// registration first just to find out.
    fn is_supported(&self) -> bool {
        true
    }
}

/// Hotkey backend for sessions with no usable global-shortcut mechanism
/// (e.g. Wayland in this version, which has no portal-based global-shortcut
/// integration - see `global-hotkey-registration`'s spec). Every
/// registration attempt reports `Unsupported` rather than hanging,
/// panicking, or silently pretending to succeed.
pub struct UnsupportedHotkeyBackend;

impl UnsupportedHotkeyBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UnsupportedHotkeyBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl HotkeyBackend for UnsupportedHotkeyBackend {
    fn register(
        &self,
        _binding: HotkeyBinding,
        _callback: Box<dyn Fn() + Send + Sync>,
    ) -> Result<(), HotkeyError> {
        Err(HotkeyError::Unsupported)
    }

    fn is_supported(&self) -> bool {
        false
    }
}

/// Real cross-desktop hotkey backend, backed by the `global-hotkey` crate.
/// Only exercised by the `#[ignore]`d manual integration test below, since it
/// needs a running desktop session to actually grab a key combination.
type HotkeyCallback = Box<dyn Fn() + Send + Sync>;
type HotkeyCallbackMap = Arc<Mutex<HashMap<u32, HotkeyCallback>>>;

pub struct GlobalHotkeyBackend {
    manager: global_hotkey::GlobalHotKeyManager,
    callbacks: HotkeyCallbackMap,
}

impl GlobalHotkeyBackend {
    pub fn new() -> Result<Self, HotkeyError> {
        let manager = global_hotkey::GlobalHotKeyManager::new()
            .map_err(|e| HotkeyError::InvalidBinding(e.to_string()))?;
        let callbacks: HotkeyCallbackMap = Arc::new(Mutex::new(HashMap::new()));

        let callbacks_for_thread = callbacks.clone();
        std::thread::spawn(move || {
            let receiver = global_hotkey::GlobalHotKeyEvent::receiver();
            while let Ok(event) = receiver.recv() {
                if event.state == global_hotkey::HotKeyState::Pressed {
                    if let Some(callback) = callbacks_for_thread.lock().unwrap().get(&event.id) {
                        callback();
                    }
                }
            }
        });

        Ok(Self { manager, callbacks })
    }
}

impl HotkeyBackend for GlobalHotkeyBackend {
    fn register(
        &self,
        binding: HotkeyBinding,
        callback: Box<dyn Fn() + Send + Sync>,
    ) -> Result<(), HotkeyError> {
        let hotkey = to_global_hotkey(binding)?;
        self.manager
            .register(hotkey)
            .map_err(|_| HotkeyError::AlreadyRegistered(binding))?;
        self.callbacks.lock().unwrap().insert(hotkey.id(), callback);
        Ok(())
    }
}

fn to_global_hotkey(binding: HotkeyBinding) -> Result<global_hotkey::hotkey::HotKey, HotkeyError> {
    use global_hotkey::hotkey::{HotKey, Modifiers};

    let mut mods = Modifiers::empty();
    if binding.ctrl {
        mods |= Modifiers::CONTROL;
    }
    if binding.shift {
        mods |= Modifiers::SHIFT;
    }
    if binding.alt {
        mods |= Modifiers::ALT;
    }
    if binding.super_key {
        mods |= Modifiers::SUPER;
    }

    let code = char_to_code(binding.key)?;
    Ok(HotKey::new(if mods.is_empty() { None } else { Some(mods) }, code))
}

fn char_to_code(ch: char) -> Result<global_hotkey::hotkey::Code, HotkeyError> {
    use global_hotkey::hotkey::Code;

    match ch {
        c @ 'A'..='Z' | c @ 'a'..='z' => {
            let name = format!("Key{}", c.to_ascii_uppercase());
            Code::from_str(&name).map_err(|_| HotkeyError::InvalidBinding(ch.to_string()))
        }
        c @ '0'..='9' => {
            let name = format!("Digit{c}");
            Code::from_str(&name).map_err(|_| HotkeyError::InvalidBinding(ch.to_string()))
        }
        '`' => Ok(Code::Backquote),
        '-' => Ok(Code::Minus),
        '=' => Ok(Code::Equal),
        '[' => Ok(Code::BracketLeft),
        ']' => Ok(Code::BracketRight),
        '\\' => Ok(Code::Backslash),
        ',' => Ok(Code::Comma),
        '.' => Ok(Code::Period),
        '\'' => Ok(Code::Quote),
        ';' => Ok(Code::Semicolon),
        '/' => Ok(Code::Slash),
        other => Err(HotkeyError::InvalidBinding(other.to_string())),
    }
}

#[cfg(test)]
mod fake {
    use super::{HotkeyBackend, HotkeyBinding, HotkeyError};
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub(crate) struct FakeHotkeyBackend {
        registered: Mutex<HashMap<HotkeyBinding, Box<dyn Fn() + Send + Sync>>>,
    }

    impl FakeHotkeyBackend {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        /// Test helper: simulates the configured binding being pressed.
        pub(crate) fn press(&self, binding: HotkeyBinding) {
            if let Some(callback) = self.registered.lock().unwrap().get(&binding) {
                callback();
            }
        }
    }

    impl HotkeyBackend for FakeHotkeyBackend {
        fn register(
            &self,
            binding: HotkeyBinding,
            callback: Box<dyn Fn() + Send + Sync>,
        ) -> Result<(), HotkeyError> {
            let mut registered = self.registered.lock().unwrap();
            if registered.contains_key(&binding) {
                return Err(HotkeyError::AlreadyRegistered(binding));
            }
            registered.insert(binding, callback);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeHotkeyBackend;
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn a_configured_binding_parses_into_the_expected_modifiers_and_key() {
        let binding = parse_binding("Ctrl+Shift+V").unwrap();
        assert!(binding.ctrl);
        assert!(binding.shift);
        assert!(!binding.alt);
        assert_eq!(binding.key, 'V');
    }

    #[test]
    fn an_invalid_binding_string_is_rejected_at_parse_time() {
        assert!(parse_binding("NotAKey+++").is_err());
    }

    #[test]
    fn a_binding_with_a_punctuation_key_parses_successfully() {
        let binding = parse_binding("Ctrl+`").unwrap();
        assert!(binding.ctrl);
        assert_eq!(binding.key, '`');
    }

    #[test]
    fn punctuation_keys_map_to_their_global_hotkey_codes() {
        assert_eq!(char_to_code('`').unwrap(), global_hotkey::hotkey::Code::Backquote);
        assert_eq!(char_to_code('-').unwrap(), global_hotkey::hotkey::Code::Minus);
        assert_eq!(char_to_code('=').unwrap(), global_hotkey::hotkey::Code::Equal);
        assert_eq!(char_to_code('[').unwrap(), global_hotkey::hotkey::Code::BracketLeft);
        assert_eq!(char_to_code(']').unwrap(), global_hotkey::hotkey::Code::BracketRight);
        assert_eq!(char_to_code('\\').unwrap(), global_hotkey::hotkey::Code::Backslash);
        assert_eq!(char_to_code(',').unwrap(), global_hotkey::hotkey::Code::Comma);
        assert_eq!(char_to_code('.').unwrap(), global_hotkey::hotkey::Code::Period);
        assert_eq!(char_to_code('\'').unwrap(), global_hotkey::hotkey::Code::Quote);
        assert_eq!(char_to_code(';').unwrap(), global_hotkey::hotkey::Code::Semicolon);
        assert_eq!(char_to_code('/').unwrap(), global_hotkey::hotkey::Code::Slash);
    }

    #[test]
    fn pressing_the_registered_combination_triggers_activation_exactly_once() {
        let backend = FakeHotkeyBackend::new();
        let binding = parse_binding("Ctrl+Shift+V").unwrap();
        let count = std::sync::Arc::new(AtomicUsize::new(0));
        let count_clone = count.clone();
        backend.register(binding, Box::new(move || { count_clone.fetch_add(1, Ordering::SeqCst); })).unwrap();

        backend.press(binding);

        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn registration_against_a_no_shortcut_mechanism_backend_returns_unsupported() {
        let backend = UnsupportedHotkeyBackend::new();
        let binding = parse_binding("Ctrl+Shift+V").unwrap();

        let result = backend.register(binding, Box::new(|| {}));

        assert!(matches!(result, Err(HotkeyError::Unsupported)));
    }

    #[test]
    fn capabilities_report_hotkeys_unsupported_for_a_backend_with_no_shortcut_mechanism() {
        let backend = UnsupportedHotkeyBackend::new();

        assert!(!backend.is_supported());
    }

    #[test]
    fn a_real_capable_backend_reports_hotkeys_supported() {
        let backend = FakeHotkeyBackend::new();

        assert!(backend.is_supported());
    }

    #[test]
    fn conflicting_registration_returns_an_error() {
        let backend = FakeHotkeyBackend::new();
        let binding = parse_binding("Ctrl+Shift+V").unwrap();
        backend.register(binding, Box::new(|| {})).unwrap();

        let result = backend.register(binding, Box::new(|| {}));

        assert!(matches!(result, Err(HotkeyError::AlreadyRegistered(_))));
    }

    /// Registers a real global hotkey and waits for a manual key press.
    /// Needs a running desktop session (not headless/Xvfb-safe, since it
    /// relies on the desktop's actual input event delivery). Run manually
    /// with `cargo test -p clip-platform -- --ignored hotkeys::real_hotkey`,
    /// then press Ctrl+Shift+Y within 10 seconds.
    #[test]
    #[ignore = "requires a running desktop session and a manual key press"]
    fn real_hotkey_registration_triggers_on_manual_key_press() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let backend = GlobalHotkeyBackend::new().expect("create global hotkey manager");
        let binding = parse_binding("Ctrl+Shift+Y").unwrap();
        let pressed = std::sync::Arc::new(AtomicBool::new(false));
        let pressed_clone = pressed.clone();
        backend.register(binding, Box::new(move || pressed_clone.store(true, Ordering::SeqCst))).unwrap();

        for _ in 0..100 {
            if pressed.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        assert!(pressed.load(Ordering::SeqCst), "Ctrl+Shift+Y was not observed within 10 seconds");
    }
}
