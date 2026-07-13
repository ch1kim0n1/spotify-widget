use std::time::Duration;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Spotify authorization is required.")]
    AuthenticationRequired,
    #[error("Spotify client ID is not configured.")]
    ConfigurationRequired,
    #[error("No active Spotify playback device is available.")]
    NoActiveDevice,
    #[error("Spotify Premium is required for playback controls.")]
    PremiumRequired,
    #[error("Spotify temporarily limited requests. Retry in {0:?}.")]
    RateLimited(Duration),
    #[error("The network is unavailable.")]
    NetworkUnavailable,
    #[error("Secure credential storage failed: {0}")]
    SecureStore(String),
    #[error("Settings storage failed: {0}")]
    Storage(String),
    #[error("Spotify returned an unsupported response: {0}")]
    UnsupportedResponse(String),
    #[error("Spotify request failed: {0}")]
    SpotifyApi(String),
    #[error("Window operation failed: {0}")]
    Window(String),
    #[error("Platform integration failed: {0}")]
    Platform(String),
    #[error("Authorization failed: {0}")]
    Authorization(String),
    #[error("Internal application error: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for AppError {
    fn from(error: reqwest::Error) -> Self {
        if error.is_connect() || error.is_timeout() {
            Self::NetworkUnavailable
        } else {
            Self::SpotifyApi(error.to_string())
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::Storage(error.to_string())
    }
}

impl From<tauri::Error> for AppError {
    fn from(error: tauri::Error) -> Self {
        Self::Platform(error.to_string())
    }
}
