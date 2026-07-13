use std::{sync::Arc, time::Duration};

use chrono::Utc;
use reqwest::{Client, Method, Response, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::{
    auth::AuthManager,
    domain::{
        CommandName, Freshness, MediaItem, MediaKind, PlaybackView, QueueAvailability, QueueView,
    },
    error::{AppError, AppResult},
};

const API_BASE: &str = "https://api.spotify.com/v1";

#[derive(Debug, Clone)]
pub struct PlaybackSnapshot {
    pub playback: PlaybackView,
    pub context_href: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawPlayback {
    device: Option<RawDevice>,
    context: Option<RawContext>,
    progress_ms: Option<u64>,
    is_playing: Option<bool>,
    item: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct RawDevice {
    name: Option<String>,
    is_active: Option<bool>,
    is_restricted: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct RawContext {
    href: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawQueue {
    queue: Option<Vec<Value>>,
}

#[derive(Debug, Deserialize)]
struct RawImage {
    url: Option<String>,
    width: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct RawCreator {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawNamedContainer {
    name: Option<String>,
    publisher: Option<String>,
    images: Option<Vec<RawImage>>,
}

#[derive(Debug, Deserialize)]
struct RawMediaItem {
    uri: Option<String>,
    #[serde(rename = "type")]
    kind: Option<String>,
    name: Option<String>,
    artists: Option<Vec<RawCreator>>,
    album: Option<RawNamedContainer>,
    show: Option<RawNamedContainer>,
    duration_ms: Option<u64>,
    is_local: Option<bool>,
    explicit: Option<bool>,
    images: Option<Vec<RawImage>>,
}

pub struct SpotifyClient {
    auth: Arc<AuthManager>,
    client: Client,
    base_url: String,
}

impl SpotifyClient {
    pub fn new(auth: Arc<AuthManager>) -> AppResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(12))
            .user_agent(format!(
                "SpotifyCompanionWidget/{}",
                env!("CARGO_PKG_VERSION")
            ))
            .build()?;
        Ok(Self {
            auth,
            client,
            base_url: API_BASE.into(),
        })
    }

    pub async fn get_playback(&self) -> AppResult<Option<PlaybackSnapshot>> {
        let response = self.request(Method::GET, "/me/player").await?;
        if response.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        }
        let response = classify_response(response).await?;
        let raw: RawPlayback = response
            .json()
            .await
            .map_err(|error| AppError::UnsupportedResponse(error.to_string()))?;
        let item = raw.item.as_ref().map(parse_media_item).transpose()?;
        let can_control = raw.device.as_ref().is_some_and(|device| {
            device.is_active.unwrap_or(true) && !device.is_restricted.unwrap_or(false)
        });
        let context_label = raw
            .context
            .as_ref()
            .and_then(|context| context.kind.as_deref())
            .map(context_fallback_label);
        Ok(Some(PlaybackSnapshot {
            playback: PlaybackView {
                item,
                is_playing: raw.is_playing,
                can_control,
                progress_ms: raw.progress_ms,
                observed_at: Utc::now(),
                freshness: Freshness::Fresh,
                context_label,
                device_name: raw.device.and_then(|device| device.name),
            },
            context_href: raw.context.and_then(|context| context.href),
        }))
    }

    pub async fn get_queue(&self) -> AppResult<QueueView> {
        let response = self.request(Method::GET, "/me/player/queue").await?;
        if response.status() == StatusCode::NO_CONTENT {
            return Ok(QueueView {
                next: None,
                availability: QueueAvailability::Unavailable,
            });
        }
        let response = classify_response(response).await?;
        let raw: RawQueue = response
            .json()
            .await
            .map_err(|error| AppError::UnsupportedResponse(error.to_string()))?;
        let next = raw
            .queue
            .unwrap_or_default()
            .into_iter()
            .next()
            .as_ref()
            .map(parse_media_item)
            .transpose()?;
        Ok(QueueView {
            availability: if next.is_some() {
                QueueAvailability::Available
            } else {
                QueueAvailability::Empty
            },
            next,
        })
    }

    pub async fn resolve_context_label(&self, href: &str) -> AppResult<Option<String>> {
        let url =
            Url::parse(href).map_err(|error| AppError::UnsupportedResponse(error.to_string()))?;
        if url.scheme() != "https"
            || url.host_str() != Some("api.spotify.com")
            || !url.path().starts_with("/v1/")
        {
            return Err(AppError::UnsupportedResponse(
                "context URL was outside the Spotify API allowlist".into(),
            ));
        }
        let token = self.auth.access_token().await?;
        let response = self.client.get(url).bearer_auth(token).send().await?;
        let response = classify_response(response).await?;
        let body: Value = response
            .json()
            .await
            .map_err(|error| AppError::UnsupportedResponse(error.to_string()))?;
        Ok(body
            .get("name")
            .or_else(|| body.get("title"))
            .and_then(Value::as_str)
            .map(str::to_owned))
    }

    pub async fn transport(&self, command: CommandName, currently_playing: bool) -> AppResult<()> {
        let (method, path) = match command {
            CommandName::Previous => (Method::POST, "/me/player/previous"),
            CommandName::Next => (Method::POST, "/me/player/next"),
            CommandName::TogglePlayPause if currently_playing => (Method::PUT, "/me/player/pause"),
            CommandName::TogglePlayPause => (Method::PUT, "/me/player/play"),
        };
        let response = self.request(method, path).await?;
        classify_response(response).await?;
        Ok(())
    }

    async fn request(&self, method: Method, path: &str) -> AppResult<Response> {
        let token = self.auth.access_token().await?;
        let url = format!("{}{path}", self.base_url);
        let retryable = matches!(method, Method::GET | Method::PUT);
        let backoffs = [Duration::from_millis(250), Duration::from_secs(1)];
        let mut attempt = 0;
        loop {
            let response = self
                .client
                .request(method.clone(), &url)
                .bearer_auth(&token)
                .send()
                .await;
            match response {
                Ok(response) if response.status() == StatusCode::UNAUTHORIZED => {
                    self.auth.invalidate_access_token().await;
                    return Err(AppError::AuthenticationRequired);
                }
                Ok(response)
                    if retryable
                        && response.status().is_server_error()
                        && attempt < backoffs.len() =>
                {
                    tokio::time::sleep(backoffs[attempt]).await;
                    attempt += 1;
                }
                Ok(response) => return Ok(response),
                Err(error)
                    if retryable
                        && (error.is_connect() || error.is_timeout())
                        && attempt < backoffs.len() =>
                {
                    tokio::time::sleep(backoffs[attempt]).await;
                    attempt += 1;
                }
                Err(error) => return Err(error.into()),
            }
        }
    }
}

async fn classify_response(response: Response) -> AppResult<Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = response.text().await.unwrap_or_default();
    Err(classify_api_error(status, retry_after.as_deref(), &body))
}

fn classify_api_error(status: StatusCode, retry_after: Option<&str>, body: &str) -> AppError {
    if status == StatusCode::TOO_MANY_REQUESTS {
        let retry_after = retry_after
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(30);
        return AppError::RateLimited(Duration::from_secs(retry_after));
    }
    if status == StatusCode::UNAUTHORIZED {
        return AppError::AuthenticationRequired;
    }
    if status == StatusCode::FORBIDDEN {
        let normalized = body.to_lowercase();
        if normalized.contains("premium") {
            return AppError::PremiumRequired;
        }
        if normalized.contains("device") || normalized.contains("player command failed") {
            return AppError::NoActiveDevice;
        }
    }
    AppError::SpotifyApi(format!(
        "Spotify returned {status}: {}",
        truncate(body, 180)
    ))
}

fn parse_media_item(value: &Value) -> AppResult<MediaItem> {
    let raw: RawMediaItem = serde_json::from_value(value.clone())
        .map_err(|error| AppError::UnsupportedResponse(error.to_string()))?;
    let is_local = raw.is_local.unwrap_or(false);
    let kind = match raw.kind.as_deref() {
        Some("track") if is_local => MediaKind::Local,
        Some("track") => MediaKind::Track,
        Some("episode") => MediaKind::Episode,
        Some("ad") | Some("advertisement") => MediaKind::Advertisement,
        _ if is_local => MediaKind::Local,
        _ => MediaKind::Unknown,
    };
    let creators = raw
        .artists
        .unwrap_or_default()
        .into_iter()
        .filter_map(|creator| creator.name)
        .chain(raw.show.as_ref().and_then(|show| show.publisher.clone()))
        .collect::<Vec<_>>();
    let album_or_show = raw
        .album
        .as_ref()
        .and_then(|album| album.name.clone())
        .or_else(|| raw.show.as_ref().and_then(|show| show.name.clone()));
    let artwork_url = best_image(
        raw.album
            .as_ref()
            .and_then(|album| album.images.as_deref())
            .or(raw.images.as_deref())
            .or_else(|| raw.show.as_ref().and_then(|show| show.images.as_deref())),
    );

    Ok(MediaItem {
        provider_uri: raw.uri,
        kind,
        title: raw.name.unwrap_or_else(|| "Unknown item".into()),
        creators,
        album_or_show,
        duration_ms: raw.duration_ms,
        artwork_url,
        is_local,
        is_explicit: raw.explicit,
    })
}

fn best_image(images: Option<&[RawImage]>) -> Option<String> {
    images?
        .iter()
        .filter_map(|image| {
            let url = image.url.as_ref()?;
            let parsed = Url::parse(url).ok()?;
            let allowed = parsed.scheme() == "https"
                && parsed
                    .host_str()
                    .is_some_and(|host| host == "i.scdn.co" || host.ends_with(".scdn.co"));
            allowed.then_some((image.width.unwrap_or_default(), url.clone()))
        })
        .min_by_key(|(width, _)| width.abs_diff(300))
        .map(|(_, url)| url)
}

fn context_fallback_label(kind: &str) -> String {
    match kind {
        "playlist" => "Playlist",
        "album" => "Album",
        "artist" => "Artist radio",
        "collection" => "Liked Songs",
        "show" => "Podcast",
        _ => "Unknown context",
    }
    .into()
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_track_with_missing_optional_fields() {
        let item = parse_media_item(&json!({
            "type": "track",
            "name": "A track",
            "artists": [{ "name": "An artist" }],
            "duration_ms": 1234,
            "is_local": false,
            "album": { "name": "An album", "images": [] }
        }))
        .expect("track should parse");
        assert_eq!(item.title, "A track");
        assert_eq!(item.kind, MediaKind::Track);
        assert_eq!(item.creators, vec!["An artist"]);
        assert_eq!(item.artwork_url, None);
    }

    #[test]
    fn parses_unknown_item_without_crashing() {
        let item = parse_media_item(&json!({ "name": "Sponsored message" }))
            .expect("unknown item should parse");
        assert_eq!(item.kind, MediaKind::Unknown);
        assert_eq!(item.title, "Sponsored message");
    }

    #[test]
    fn classifies_auth_rate_limit_and_device_failures() {
        assert!(matches!(
            classify_api_error(StatusCode::UNAUTHORIZED, None, ""),
            AppError::AuthenticationRequired
        ));
        assert!(matches!(
            classify_api_error(StatusCode::TOO_MANY_REQUESTS, Some("17"), ""),
            AppError::RateLimited(delay) if delay == Duration::from_secs(17)
        ));
        assert!(matches!(
            classify_api_error(StatusCode::FORBIDDEN, None, "No active device"),
            AppError::NoActiveDevice
        ));
        assert!(matches!(
            classify_api_error(StatusCode::INTERNAL_SERVER_ERROR, None, "upstream"),
            AppError::SpotifyApi(_)
        ));
    }
}
