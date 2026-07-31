//! GNOME GSettings/DConf custom-keybinding hotkey backend, for sessions
//! (GNOME/Mutter Wayland) where `XGrabKey`-based registration
//! (`GlobalHotkeyBackend`) succeeds at the X server but the compositor never
//! actually delivers the key event. GNOME Shell itself owns the shortcut and
//! runs an external trigger command on keypress instead.

use crate::hotkeys::{HotkeyBackend, HotkeyBinding, HotkeyError};

const KEYBINDINGS_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const KEYBINDINGS_KEY: &str = "custom-keybindings";
const CHILD_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const ENTRY_PATH: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/clipdeck/";
const ENTRY_NAME: &str = "ClipDeck";

/// Seam over the `gsettings` CLI, so tests can assert exact invocations
/// without touching real dconf state.
pub trait GSettingsRunner: Send + Sync {
    /// `schema` may include a `:PATH` suffix for relocatable schemas, e.g.
    /// `"org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/.../clipdeck/"`.
    fn get(&self, schema: &str, key: &str) -> Result<String, HotkeyError>;
    fn set(&self, schema: &str, key: &str, value: &str) -> Result<(), HotkeyError>;
}

pub struct GSettingsHotkeyBackend<R: GSettingsRunner> {
    runner: R,
    trigger_command: String,
}

impl<R: GSettingsRunner> GSettingsHotkeyBackend<R> {
    pub fn new(runner: R, trigger_command: impl Into<String>) -> Self {
        Self { runner, trigger_command: trigger_command.into() }
    }
}

/// Real `GSettingsRunner`, shelling out to the `gsettings` CLI (part of
/// `gnome-settings-daemon`, present on any GNOME session).
pub struct RealGSettingsRunner;

impl RealGSettingsRunner {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RealGSettingsRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl GSettingsRunner for RealGSettingsRunner {
    fn get(&self, schema: &str, key: &str) -> Result<String, HotkeyError> {
        let output = std::process::Command::new("gsettings")
            .args(["get", schema, key])
            .output()
            .map_err(|e| HotkeyError::GSettingsFailure(e.to_string()))?;
        if !output.status.success() {
            return Err(HotkeyError::GSettingsFailure(String::from_utf8_lossy(&output.stderr).trim().to_string()));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn set(&self, schema: &str, key: &str, value: &str) -> Result<(), HotkeyError> {
        let output = std::process::Command::new("gsettings")
            .args(["set", schema, key, value])
            .output()
            .map_err(|e| HotkeyError::GSettingsFailure(e.to_string()))?;
        if !output.status.success() {
            return Err(HotkeyError::GSettingsFailure(String::from_utf8_lossy(&output.stderr).trim().to_string()));
        }
        Ok(())
    }
}

/// Parses the string representation `gsettings get` prints for an array of
/// strings (e.g. `"@as []"`, `"[]"`, or `"['/a/', '/b/']"`) into its elements.
fn parse_string_array(raw: &str) -> Vec<String> {
    let inner = raw.trim().trim_start_matches("@as").trim();
    let inner = inner.trim_start_matches('[').trim_end_matches(']').trim();
    if inner.is_empty() {
        return Vec::new();
    }
    inner
        .split(',')
        .map(|item| item.trim().trim_matches('\'').to_string())
        .collect()
}

fn format_string_array(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|item| format!("'{item}'")).collect();
    format!("[{}]", quoted.join(", "))
}

impl<R: GSettingsRunner> HotkeyBackend for GSettingsHotkeyBackend<R> {
    fn register(&self, binding: HotkeyBinding, _callback: Box<dyn Fn() + Send + Sync>) -> Result<(), HotkeyError> {
        let existing = self.runner.get(KEYBINDINGS_SCHEMA, KEYBINDINGS_KEY)?;
        let mut paths = parse_string_array(&existing);
        if !paths.iter().any(|path| path == ENTRY_PATH) {
            paths.push(ENTRY_PATH.to_string());
            self.runner.set(KEYBINDINGS_SCHEMA, KEYBINDINGS_KEY, &format_string_array(&paths))?;
        }

        let child_schema = format!("{CHILD_SCHEMA}:{ENTRY_PATH}");
        self.runner.set(&child_schema, "name", &format!("'{ENTRY_NAME}'"))?;
        self.runner.set(&child_schema, "command", &format!("'{}'", self.trigger_command))?;
        self.runner.set(&child_schema, "binding", &format!("'{}'", binding_to_accelerator(&binding)))?;
        Ok(())
    }
}

fn binding_to_accelerator(binding: &HotkeyBinding) -> String {
    let mut accelerator = String::new();
    if binding.ctrl {
        accelerator.push_str("<Control>");
    }
    if binding.shift {
        accelerator.push_str("<Shift>");
    }
    if binding.alt {
        accelerator.push_str("<Alt>");
    }
    if binding.super_key {
        accelerator.push_str("<Super>");
    }
    accelerator.push_str(&keysym_name(binding.key.to_ascii_lowercase()));
    accelerator
}

/// GTK's accelerator parser (used by GNOME's custom-keybindings) expects the
/// X11/GDK keysym *name* for punctuation keys (e.g. `"grave"`, not the
/// literal `` ` `` character) - unlike letters/digits, where the literal
/// character is the correct and accepted form.
fn keysym_name(key: char) -> String {
    match key {
        '`' => "grave".to_string(),
        '-' => "minus".to_string(),
        '=' => "equal".to_string(),
        '[' => "bracketleft".to_string(),
        ']' => "bracketright".to_string(),
        '\\' => "backslash".to_string(),
        ',' => "comma".to_string(),
        '.' => "period".to_string(),
        '\'' => "apostrophe".to_string(),
        ';' => "semicolon".to_string(),
        '/' => "slash".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod fake {
    use super::GSettingsRunner;
    use crate::hotkeys::HotkeyError;
    use std::sync::Mutex;

    #[derive(Default)]
    pub(crate) struct FakeGSettingsRunner {
        get_responses: Mutex<std::collections::HashMap<(String, String), String>>,
        set_calls: Mutex<Vec<(String, String, String)>>,
    }

    impl FakeGSettingsRunner {
        pub(crate) fn new() -> Self {
            Self::default()
        }

        pub(crate) fn set_get_response(&self, schema: &str, key: &str, value: &str) {
            self.get_responses.lock().unwrap().insert((schema.to_string(), key.to_string()), value.to_string());
        }

        pub(crate) fn set_calls(&self) -> Vec<(String, String, String)> {
            self.set_calls.lock().unwrap().clone()
        }
    }

    impl GSettingsRunner for FakeGSettingsRunner {
        fn get(&self, schema: &str, key: &str) -> Result<String, HotkeyError> {
            Ok(self
                .get_responses
                .lock()
                .unwrap()
                .get(&(schema.to_string(), key.to_string()))
                .cloned()
                .unwrap_or_else(|| "@as []".to_string()))
        }

        fn set(&self, schema: &str, key: &str, value: &str) -> Result<(), HotkeyError> {
            self.set_calls.lock().unwrap().push((schema.to_string(), key.to_string(), value.to_string()));
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeGSettingsRunner;
    use super::*;
    use crate::hotkeys::parse_binding;

    #[test]
    fn register_writes_a_custom_keybinding_pointing_at_the_trigger_command() {
        let runner = FakeGSettingsRunner::new();
        runner.set_get_response(KEYBINDINGS_SCHEMA, KEYBINDINGS_KEY, "@as []");
        let backend = GSettingsHotkeyBackend::new(runner, "/usr/local/bin/clip-hotkey-trigger");
        let binding = parse_binding("Ctrl+Shift+V").unwrap();

        let result = backend.register(binding, Box::new(|| {}));

        assert!(result.is_ok());
        let child_schema = format!("{CHILD_SCHEMA}:{ENTRY_PATH}");
        assert_eq!(
            backend.runner.set_calls(),
            vec![
                (KEYBINDINGS_SCHEMA.to_string(), KEYBINDINGS_KEY.to_string(), format!("['{ENTRY_PATH}']")),
                (child_schema.clone(), "name".to_string(), format!("'{ENTRY_NAME}'")),
                (child_schema.clone(), "command".to_string(), "'/usr/local/bin/clip-hotkey-trigger'".to_string()),
                (child_schema, "binding".to_string(), "'<Control><Shift>v'".to_string()),
            ]
        );
    }

    #[test]
    fn registration_is_idempotent_across_repeated_calls() {
        let runner = FakeGSettingsRunner::new();
        runner.set_get_response(KEYBINDINGS_SCHEMA, KEYBINDINGS_KEY, &format!("['{ENTRY_PATH}']"));
        let backend = GSettingsHotkeyBackend::new(runner, "/usr/local/bin/clip-hotkey-trigger");
        let binding = parse_binding("Ctrl+Shift+V").unwrap();

        backend.register(binding, Box::new(|| {})).unwrap();
        backend.register(binding, Box::new(|| {})).unwrap();

        let keybindings_list_writes: Vec<_> = backend
            .runner
            .set_calls()
            .into_iter()
            .filter(|(schema, key, _)| schema == KEYBINDINGS_SCHEMA && key == KEYBINDINGS_KEY)
            .collect();
        assert!(
            keybindings_list_writes.is_empty(),
            "the keybindings list should not be rewritten when the entry is already present"
        );
    }

    #[test]
    fn punctuation_keys_translate_to_their_gtk_accelerator_keysym_names() {
        let runner = FakeGSettingsRunner::new();
        runner.set_get_response(KEYBINDINGS_SCHEMA, KEYBINDINGS_KEY, "@as []");
        let backend = GSettingsHotkeyBackend::new(runner, "/usr/local/bin/clip-hotkey-trigger");
        let binding = parse_binding("Ctrl+`").unwrap();

        backend.register(binding, Box::new(|| {})).unwrap();

        let child_schema = format!("{CHILD_SCHEMA}:{ENTRY_PATH}");
        let binding_write = backend
            .runner
            .set_calls()
            .into_iter()
            .find(|(schema, key, _)| schema == &child_schema && key == "binding")
            .expect("binding should have been written");
        assert_eq!(binding_write.2, "'<Control>grave'");
    }

    #[test]
    fn is_supported_returns_true() {
        let runner = FakeGSettingsRunner::new();
        let backend = GSettingsHotkeyBackend::new(runner, "/usr/local/bin/clip-hotkey-trigger");

        assert!(backend.is_supported());
    }
}
