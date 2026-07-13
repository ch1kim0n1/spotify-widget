use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use atomic_write_file::AtomicWriteFile;
use chrono::{DateTime, Utc};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use uuid::Uuid;

use crate::{
    domain::AppSettings,
    error::{AppError, AppResult},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionCheckpoint {
    pub schema_version: u32,
    pub session_id: Uuid,
    pub platform_process_identity: String,
    pub started_at: DateTime<Utc>,
    pub spotify_open_ms: u64,
    pub active_listening_ms: u64,
    pub last_checkpoint_at: DateTime<Utc>,
    pub was_playing: bool,
}

#[derive(Debug, Clone)]
pub struct Storage {
    config_dir: PathBuf,
    data_dir: PathBuf,
}

impl Storage {
    pub fn new() -> AppResult<Self> {
        let project = ProjectDirs::from("com", "vladislav", "Spotify Companion Widget")
            .ok_or_else(|| {
                AppError::Storage("operating-system data directories are unavailable".into())
            })?;
        let config_dir = project.config_dir().to_path_buf();
        let data_dir = project.data_local_dir().to_path_buf();
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(data_dir.join("logs"))?;
        Ok(Self {
            config_dir,
            data_dir,
        })
    }

    pub fn settings_path(&self) -> PathBuf {
        self.config_dir.join("settings.json")
    }

    pub fn session_path(&self) -> PathBuf {
        self.data_dir.join("session.json")
    }

    pub fn log_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn load_settings(&self) -> AppSettings {
        self.load_or_default(&self.settings_path())
    }

    pub fn save_settings(&self, settings: &AppSettings) -> AppResult<()> {
        self.save_json(&self.settings_path(), settings)
    }

    pub fn load_session(&self) -> Option<SessionCheckpoint> {
        self.load_json(&self.session_path()).ok().flatten()
    }

    pub fn save_session(&self, checkpoint: &SessionCheckpoint) -> AppResult<()> {
        self.save_json(&self.session_path(), checkpoint)
    }

    pub fn clear_session(&self) -> AppResult<()> {
        remove_if_exists(&self.session_path())
    }

    pub fn reset_local_files(&self) -> AppResult<()> {
        remove_if_exists(&self.settings_path())?;
        remove_if_exists(&self.session_path())?;
        remove_dir_contents(&self.log_dir())?;
        remove_dir_contents(&self.data_dir.join("artwork-cache"))
    }

    fn load_or_default<T>(&self, path: &Path) -> T
    where
        T: DeserializeOwned + Default,
    {
        match self.load_json(path) {
            Ok(Some(value)) => value,
            Ok(None) => T::default(),
            Err(error) => {
                tracing::warn!(%error, path = %path.display(), "local data is corrupt; using defaults");
                let backup =
                    path.with_extension(format!("corrupt-{}.json", Utc::now().timestamp()));
                if let Err(rename_error) = fs::rename(path, &backup) {
                    tracing::warn!(%rename_error, "could not preserve corrupt settings");
                }
                T::default()
            }
        }
    }

    fn load_json<T: DeserializeOwned>(&self, path: &Path) -> AppResult<Option<T>> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|error| AppError::Storage(error.to_string())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn save_json<T: Serialize>(&self, path: &Path, value: &T) -> AppResult<()> {
        let bytes = serde_json::to_vec_pretty(value)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let mut file = AtomicWriteFile::options()
            .open(path)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.commit()
            .map_err(|error| AppError::Storage(error.to_string()))
    }
}

fn remove_if_exists(path: &Path) -> AppResult<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn remove_dir_contents(path: &Path) -> AppResult<()> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            fs::remove_dir_all(entry_path)?;
        } else {
            fs::remove_file(entry_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_storage() -> (tempfile::TempDir, Storage) {
        let root = tempfile::tempdir().expect("temporary directory");
        let storage = Storage {
            config_dir: root.path().join("config"),
            data_dir: root.path().join("data"),
        };
        fs::create_dir_all(&storage.config_dir).expect("config directory");
        fs::create_dir_all(storage.log_dir()).expect("log directory");
        (root, storage)
    }

    #[test]
    fn round_trips_settings_through_atomic_file() {
        let (_root, storage) = temporary_storage();
        let mut settings = AppSettings::default();
        settings.startup.launch_at_login = true;
        storage.save_settings(&settings).expect("save settings");
        assert_eq!(storage.load_settings(), settings);
    }

    #[test]
    fn corrupt_settings_are_preserved_and_replaced_with_defaults() {
        let (_root, storage) = temporary_storage();
        fs::write(storage.settings_path(), b"{not-json").expect("write corrupt settings");
        assert_eq!(storage.load_settings(), AppSettings::default());
        let backups = fs::read_dir(&storage.config_dir)
            .expect("read config directory")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("settings.corrupt-")
            })
            .count();
        assert_eq!(backups, 1);
    }

    #[test]
    fn fills_defaults_when_loading_an_older_partial_settings_file() {
        let (_root, storage) = temporary_storage();
        fs::write(
            storage.settings_path(),
            br#"{"schemaVersion":1,"window":{"alwaysOnTop":false}}"#,
        )
        .expect("write partial settings");
        let settings = storage.load_settings();
        assert!(!settings.window.always_on_top);
        assert!(settings.display.show_listening_time);
        assert!(settings.playback.prefer_local_windows_controls);
    }
}
