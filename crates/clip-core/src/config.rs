//! Settings model and defaults.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::PasteMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsKey {
    HotkeyBinding,
    RetentionWindowDays,
    CapturePaused,
    DefaultPasteMode,
}

impl SettingsKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            SettingsKey::HotkeyBinding => "hotkey_binding",
            SettingsKey::RetentionWindowDays => "retention_window_days",
            SettingsKey::CapturePaused => "capture_paused",
            SettingsKey::DefaultPasteMode => "default_paste_mode",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey_binding: String,
    pub retention_window_days: Option<u32>,
    pub capture_paused: bool,
    pub default_paste_mode: PasteMode,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey_binding: "Ctrl+Shift+V".to_string(),
            retention_window_days: None,
            capture_paused: false,
            default_paste_mode: PasteMode::Auto,
        }
    }
}

impl AppSettings {
    /// Serializes a single field to a JSON value keyed by its `SettingsKey`.
    pub fn get_value(&self, key: SettingsKey) -> serde_json::Value {
        match key {
            SettingsKey::HotkeyBinding => serde_json::to_value(&self.hotkey_binding),
            SettingsKey::RetentionWindowDays => serde_json::to_value(self.retention_window_days),
            SettingsKey::CapturePaused => serde_json::to_value(self.capture_paused),
            SettingsKey::DefaultPasteMode => serde_json::to_value(self.default_paste_mode),
        }
        .expect("settings fields are always serializable")
    }

    /// Builds settings from a key/value source, falling back to defaults for
    /// any key that is absent.
    pub fn from_entries(entries: &HashMap<String, serde_json::Value>) -> Self {
        let mut settings = Self::default();
        if let Some(v) = entries.get(SettingsKey::HotkeyBinding.as_str()) {
            if let Ok(v) = serde_json::from_value(v.clone()) {
                settings.hotkey_binding = v;
            }
        }
        if let Some(v) = entries.get(SettingsKey::RetentionWindowDays.as_str()) {
            if let Ok(v) = serde_json::from_value(v.clone()) {
                settings.retention_window_days = v;
            }
        }
        if let Some(v) = entries.get(SettingsKey::CapturePaused.as_str()) {
            if let Ok(v) = serde_json::from_value(v.clone()) {
                settings.capture_paused = v;
            }
        }
        if let Some(v) = entries.get(SettingsKey::DefaultPasteMode.as_str()) {
            if let Ok(v) = serde_json::from_value(v.clone()) {
                settings.default_paste_mode = v;
            }
        }
        settings
    }
}

/// Resolved config/data/cache directories for the application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl AppPaths {
    /// Resolves the application's standard directories via `directories`,
    /// or under `CLIPDECK_TEST_HOME` if that environment variable is set.
    pub fn resolve() -> Self {
        if let Ok(root) = std::env::var("CLIPDECK_TEST_HOME") {
            let root = PathBuf::from(root);
            return Self {
                config_dir: root.join("config"),
                data_dir: root.join("data"),
                cache_dir: root.join("cache"),
            };
        }
        let dirs = directories::ProjectDirs::from("dev", "ClipDeck", "clipdeck")
            .expect("could not resolve a home directory for the current user");
        Self {
            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_have_capture_enabled() {
        assert!(!AppSettings::default().capture_paused);
    }

    #[test]
    fn default_settings_have_no_retention_window() {
        assert_eq!(AppSettings::default().retention_window_days, None);
    }

    #[test]
    fn a_single_setting_serializes_to_a_json_value_under_its_key() {
        let settings = AppSettings { retention_window_days: Some(30), ..AppSettings::default() };
        let value = settings.get_value(SettingsKey::RetentionWindowDays);
        let round_tripped: Option<u32> = serde_json::from_value(value).unwrap();
        assert_eq!(round_tripped, settings.retention_window_days);
    }

    #[test]
    fn missing_key_falls_back_to_the_fields_default() {
        let entries: std::collections::HashMap<String, serde_json::Value> = std::collections::HashMap::new();
        let settings = AppSettings::from_entries(&entries);
        assert_eq!(settings.hotkey_binding, AppSettings::default().hotkey_binding);
    }

    #[test]
    fn app_paths_resolve_exposes_distinct_config_data_and_cache_directories() {
        let paths = AppPaths::resolve();
        assert!(!paths.config_dir.as_os_str().is_empty());
        assert!(!paths.data_dir.as_os_str().is_empty());
        assert!(!paths.cache_dir.as_os_str().is_empty());
        assert_ne!(paths.config_dir, paths.data_dir);
        assert_ne!(paths.data_dir, paths.cache_dir);
    }

    #[test]
    fn app_paths_resolve_is_overridable_for_tests() {
        std::env::set_var("CLIPDECK_TEST_HOME", "/tmp/clipdeck-test-home-override");
        let paths = AppPaths::resolve();
        std::env::remove_var("CLIPDECK_TEST_HOME");
        assert!(paths.config_dir.starts_with("/tmp/clipdeck-test-home-override"));
        assert!(paths.data_dir.starts_with("/tmp/clipdeck-test-home-override"));
        assert!(paths.cache_dir.starts_with("/tmp/clipdeck-test-home-override"));
    }
}
