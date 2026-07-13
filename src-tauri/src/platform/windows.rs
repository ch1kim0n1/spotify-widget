use async_trait::async_trait;
use windows::{
    Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
    core::HSTRING,
};

use crate::{
    domain::CommandName,
    error::{AppError, AppResult},
    platform::{LocalMediaSnapshot, PlatformMediaAdapter},
};

#[derive(Debug, Default)]
pub struct WindowsMediaAdapter;

#[async_trait]
impl PlatformMediaAdapter for WindowsMediaAdapter {
    async fn snapshot(&self) -> AppResult<Option<LocalMediaSnapshot>> {
        snapshot_async().await
    }

    async fn transport(&self, command: CommandName) -> AppResult<bool> {
        transport_async(command).await
    }
}

async fn snapshot_async() -> AppResult<Option<LocalMediaSnapshot>> {
    let Some(session) = spotify_session().await? else {
        return Ok(None);
    };
    let properties = session
        .TryGetMediaPropertiesAsync()
        .map_err(winrt_error)?
        .await
        .map_err(winrt_error)?;
    let playback = session.GetPlaybackInfo().map_err(winrt_error)?;
    let timeline = session.GetTimelineProperties().ok();
    let title = hstring_to_string(properties.Title().unwrap_or_default());
    let artist = hstring_to_string(properties.Artist().unwrap_or_default());
    if title.is_empty() && artist.is_empty() {
        return Ok(None);
    }
    let album = hstring_to_string(properties.AlbumTitle().unwrap_or_default());
    let is_playing = playback.PlaybackStatus().map_err(winrt_error)?
        == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;
    let progress_ms = timeline
        .as_ref()
        .and_then(|value| value.Position().ok())
        .and_then(timespan_ms);
    let duration_ms = timeline
        .as_ref()
        .and_then(|value| value.EndTime().ok())
        .and_then(timespan_ms);

    Ok(Some(LocalMediaSnapshot {
        title,
        artist,
        album: (!album.is_empty()).then_some(album),
        is_playing,
        progress_ms,
        duration_ms,
    }))
}

async fn transport_async(command: CommandName) -> AppResult<bool> {
    let session = spotify_session()
        .await?
        .ok_or_else(|| AppError::Platform("Spotify media session is unavailable".into()))?;
    let accepted = match command {
        CommandName::Previous => session.TrySkipPreviousAsync().map_err(winrt_error)?.await,
        CommandName::TogglePlayPause => {
            session
                .TryTogglePlayPauseAsync()
                .map_err(winrt_error)?
                .await
        }
        CommandName::Next => session.TrySkipNextAsync().map_err(winrt_error)?.await,
    }
    .map_err(winrt_error)?;
    Ok(accepted)
}

async fn spotify_session() -> AppResult<Option<GlobalSystemMediaTransportControlsSession>> {
    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(winrt_error)?
        .await
        .map_err(winrt_error)?;
    let sessions = manager.GetSessions().map_err(winrt_error)?;
    for index in 0..sessions.Size().map_err(winrt_error)? {
        let session = sessions.GetAt(index).map_err(winrt_error)?;
        let source = session.SourceAppUserModelId().map_err(winrt_error)?;
        if is_spotify_source(&source) {
            return Ok(Some(session));
        }
    }
    let current = manager.GetCurrentSession().ok();
    Ok(current.filter(|session| {
        session
            .SourceAppUserModelId()
            .is_ok_and(|source| is_spotify_source(&source))
    }))
}

fn is_spotify_source(source: &HSTRING) -> bool {
    let source = source.to_string_lossy().to_ascii_lowercase();
    source.contains("spotify") && !source.contains("chrome") && !source.contains("edge")
}

fn hstring_to_string(value: HSTRING) -> String {
    value.to_string_lossy().trim().to_owned()
}

fn timespan_ms(value: windows::Foundation::TimeSpan) -> Option<u64> {
    u64::try_from(value.Duration)
        .ok()
        .map(|ticks| ticks / 10_000)
}

fn winrt_error(error: windows::core::Error) -> AppError {
    AppError::Platform(error.to_string())
}
