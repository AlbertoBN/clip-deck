//! Persists the popup window's last on-screen position across restores (app
//! restarts, and every show - see `run()`'s `HotkeyPressed` handling), since
//! it otherwise always reopens at the platform's default origin instead of
//! wherever the user last dragged it to.

use std::path::{Path, PathBuf};

#[derive(serde::Serialize, serde::Deserialize)]
struct Position {
    x: i32,
    y: i32,
}

fn file_path(config_dir: &Path) -> PathBuf {
    config_dir.join("popup_position.json")
}

/// Persists `(x, y)` to `<config_dir>/popup_position.json`. Failures (e.g. a
/// non-writable config dir) are silently ignored - losing the remembered
/// position is a minor UX regression, not worth surfacing an error for.
pub fn save(config_dir: &Path, position: (i32, i32)) {
    let path = file_path(config_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(&Position { x: position.0, y: position.1 }) {
        let _ = std::fs::write(path, json);
    }
}

/// Reads back the last position saved via `save`, or `None` if it was never
/// saved (first run) or the file is missing/corrupt.
pub fn load(config_dir: &Path) -> Option<(i32, i32)> {
    let contents = std::fs::read_to_string(file_path(config_dir)).ok()?;
    let position: Position = serde_json::from_str(&contents).ok()?;
    Some((position.x, position.y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_position_round_trips_through_load() {
        let dir = tempfile::tempdir().unwrap();

        save(dir.path(), (123, 456));

        assert_eq!(load(dir.path()), Some((123, 456)));
    }

    #[test]
    fn loading_before_anything_was_ever_saved_returns_none() {
        let dir = tempfile::tempdir().unwrap();

        assert_eq!(load(dir.path()), None);
    }

    #[test]
    fn loading_a_corrupt_file_returns_none_instead_of_panicking() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("popup_position.json"), "not json").unwrap();

        assert_eq!(load(dir.path()), None);
    }

    #[test]
    fn saving_again_overwrites_the_previous_position() {
        let dir = tempfile::tempdir().unwrap();
        save(dir.path(), (1, 2));

        save(dir.path(), (3, 4));

        assert_eq!(load(dir.path()), Some((3, 4)));
    }

    #[test]
    fn save_creates_the_config_directory_if_it_does_not_exist_yet() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("nested/config/dir");

        save(&config_dir, (7, 8));

        assert_eq!(load(&config_dir), Some((7, 8)));
    }
}
