use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use color_eyre::eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

/// Persisted user settings. `#[serde(default)]` lets older files load after
/// fields are added; give every new field a sensible default.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub show_tips: bool,
    pub counter: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_tips: true,
            counter: 0,
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        Self::load_from(&crate::app_paths::settings_path()?)
    }

    pub fn save(&self) -> Result<()> {
        self.save_to(&crate::app_paths::settings_path()?)
    }

    /// A missing file yields defaults. An unreadable file is moved aside to
    /// `settings.json.corrupt` so the next save cannot silently destroy it.
    pub fn load_from(path: &Path) -> Result<Self> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => {
                return Err(error).wrap_err_with(|| format!("could not read {}", path.display()));
            }
        };
        match serde_json::from_slice(&bytes) {
            Ok(settings) => Ok(settings),
            Err(error) => {
                let backup = path.with_extension("json.corrupt");
                tracing::warn!(%error, backup = %backup.display(), "settings are unreadable; using defaults");
                fs::rename(path, &backup)
                    .wrap_err_with(|| format!("could not move aside {}", path.display()))?;
                Ok(Self::default())
            }
        }
    }

    /// Writes a sibling temporary file and renames it over the target, so a
    /// crash mid-write never leaves a truncated settings file.
    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(self)?)?;
        fs::rename(&temporary, path)
            .wrap_err_with(|| format!("could not replace {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn scratch_dir() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "starter-settings-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_file_loads_defaults() {
        let path = scratch_dir().join("settings.json");
        assert_eq!(Settings::load_from(&path).unwrap(), Settings::default());
    }

    #[test]
    fn saved_settings_round_trip() {
        let path = scratch_dir().join("nested/settings.json");
        let settings = Settings {
            show_tips: false,
            counter: 42,
        };
        settings.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap(), settings);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn missing_and_unknown_fields_are_tolerated() {
        let path = scratch_dir().join("settings.json");
        fs::write(&path, r#"{ "counter": 7, "removed_field": true }"#).unwrap();
        let settings = Settings::load_from(&path).unwrap();
        assert_eq!(settings.counter, 7);
        assert!(settings.show_tips);
    }

    #[test]
    fn corrupt_file_is_preserved_and_defaults_are_used() {
        let path = scratch_dir().join("settings.json");
        fs::write(&path, "{ not json").unwrap();
        assert_eq!(Settings::load_from(&path).unwrap(), Settings::default());
        assert!(!path.exists());
        assert_eq!(
            fs::read_to_string(path.with_extension("json.corrupt")).unwrap(),
            "{ not json"
        );
    }
}
