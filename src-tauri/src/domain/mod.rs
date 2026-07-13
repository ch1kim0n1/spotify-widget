use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Availability {
    Unknown,
    ConfigurationRequired,
    SpotifyClosed,
    AuthenticationRequired,
    NoActiveDevice,
    Ready,
    RateLimited,
    Offline,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Track,
    Episode,
    Advertisement,
    Local,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub provider_uri: Option<String>,
    pub kind: MediaKind,
    pub title: String,
    pub creators: Vec<String>,
    pub album_or_show: Option<String>,
    pub duration_ms: Option<u64>,
    pub artwork_url: Option<String>,
    pub is_local: bool,
    pub is_explicit: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackView {
    pub item: Option<MediaItem>,
    pub is_playing: Option<bool>,
    pub can_control: bool,
    pub progress_ms: Option<u64>,
    pub observed_at: DateTime<Utc>,
    pub freshness: Freshness,
    pub context_label: Option<String>,
    pub device_name: Option<String>,
}

impl Default for PlaybackView {
    fn default() -> Self {
        Self {
            item: None,
            is_playing: None,
            can_control: false,
            progress_ms: None,
            observed_at: Utc::now(),
            freshness: Freshness::Unknown,
            context_label: None,
            device_name: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum QueueAvailability {
    Available,
    Empty,
    Unavailable,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QueueView {
    pub next: Option<MediaItem>,
    pub availability: QueueAvailability,
}

impl Default for QueueView {
    fn default() -> Self {
        Self {
            next: None,
            availability: QueueAvailability::Unavailable,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub spotify_open_ms: u64,
    pub active_listening_ms: u64,
    pub spotify_running: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CommandName {
    Previous,
    TogglePlayPause,
    Next,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommandView {
    pub pending: Option<CommandName>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StatusTone {
    Neutral,
    Positive,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub always_on_top: bool,
    pub launch_at_login: bool,
    pub show_spotify_open_time: bool,
    pub show_listening_time: bool,
}

impl Default for SettingsView {
    fn default() -> Self {
        Self {
            always_on_top: true,
            launch_at_login: false,
            show_spotify_open_time: true,
            show_listening_time: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ViewState {
    pub availability: Availability,
    pub playback: PlaybackView,
    pub queue: QueueView,
    pub session: SessionView,
    pub command: CommandView,
    pub settings: SettingsView,
    pub status_message: Option<String>,
    pub status_tone: StatusTone,
    pub revision: u64,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            availability: Availability::Unknown,
            playback: PlaybackView::default(),
            queue: QueueView::default(),
            session: SessionView::default(),
            command: CommandView::default(),
            settings: SettingsView::default(),
            status_message: Some("Starting companion…".into()),
            status_tone: StatusTone::Neutral,
            revision: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct WindowSettings {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub monitor_id: Option<String>,
    pub always_on_top: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            monitor_id: None,
            always_on_top: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct StartupSettings {
    pub launch_at_login: bool,
    pub show_widget_at_launch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct DisplaySettings {
    pub theme: String,
    pub show_spotify_open_time: bool,
    pub show_listening_time: bool,
    pub reduced_motion: String,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            show_spotify_open_time: true,
            show_listening_time: true,
            reduced_motion: "system".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct PlaybackSettings {
    pub prefer_local_windows_controls: bool,
    pub account_wide_listening_timer: bool,
}

impl Default for PlaybackSettings {
    fn default() -> Self {
        Self {
            prefer_local_windows_controls: true,
            account_wide_listening_timer: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct AppSettings {
    pub schema_version: u32,
    pub window: WindowSettings,
    pub startup: StartupSettings,
    pub display: DisplaySettings,
    pub playback: PlaybackSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: 1,
            window: WindowSettings::default(),
            startup: StartupSettings::default(),
            display: DisplaySettings::default(),
            playback: PlaybackSettings::default(),
        }
    }
}

impl From<&AppSettings> for SettingsView {
    fn from(settings: &AppSettings) -> Self {
        Self {
            always_on_top: settings.window.always_on_top,
            launch_at_login: settings.startup.launch_at_login,
            show_spotify_open_time: settings.display.show_spotify_open_time,
            show_listening_time: settings.display.show_listening_time,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    pub always_on_top: Option<bool>,
    pub launch_at_login: Option<bool>,
    pub show_spotify_open_time: Option<bool>,
    pub show_listening_time: Option<bool>,
}
