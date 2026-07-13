use async_trait::async_trait;
use sysinfo::{ProcessesToUpdate, System};

#[cfg(not(windows))]
use crate::error::AppError;
use crate::{domain::CommandName, error::AppResult};

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessObservation {
    pub running: bool,
    pub identity: Option<String>,
}

pub struct SpotifyProcessMonitor {
    system: System,
}

impl SpotifyProcessMonitor {
    pub fn new() -> Self {
        Self {
            system: System::new(),
        }
    }

    pub fn observe(&mut self) -> ProcessObservation {
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        let mut spotify_processes = self
            .system
            .processes()
            .iter()
            .filter_map(|(pid, process)| {
                let name = process.name().to_string_lossy().to_ascii_lowercase();
                let executable = process
                    .exe()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().to_ascii_lowercase())
                    .unwrap_or_default();
                let recognized = if cfg!(windows) {
                    name == "spotify.exe" || executable == "spotify.exe"
                } else {
                    name == "spotify" || executable == "spotify"
                };
                recognized.then_some((process.start_time(), pid.as_u32()))
            })
            .collect::<Vec<_>>();
        spotify_processes.sort_unstable();
        let identity = spotify_processes
            .first()
            .map(|(started, pid)| format!("{}:spotify:{pid}:{started}", std::env::consts::OS));
        ProcessObservation {
            running: identity.is_some(),
            identity,
        }
    }
}

impl Default for SpotifyProcessMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalMediaSnapshot {
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub is_playing: bool,
    pub progress_ms: Option<u64>,
    pub duration_ms: Option<u64>,
}

#[async_trait]
pub trait PlatformMediaAdapter: Send + Sync {
    async fn snapshot(&self) -> AppResult<Option<LocalMediaSnapshot>>;
    async fn transport(&self, command: CommandName) -> AppResult<bool>;
}

#[cfg(not(windows))]
#[derive(Debug, Default)]
pub struct NoLocalMediaAdapter;

#[cfg(not(windows))]
#[async_trait]
impl PlatformMediaAdapter for NoLocalMediaAdapter {
    async fn snapshot(&self) -> AppResult<Option<LocalMediaSnapshot>> {
        Ok(None)
    }

    async fn transport(&self, _command: CommandName) -> AppResult<bool> {
        Err(AppError::Platform(
            "local media controls are unavailable on this platform".into(),
        ))
    }
}
