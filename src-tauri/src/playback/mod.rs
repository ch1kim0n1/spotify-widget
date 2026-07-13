use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use chrono::Utc;
use tauri::{Emitter, Manager};
use tokio::sync::{Mutex, RwLock};

use crate::{
    auth::AuthManager,
    domain::{
        AppSettings, Availability, CommandName, Freshness, MediaItem, MediaKind, QueueAvailability,
        StatusTone, ViewState,
    },
    error::{AppError, AppResult},
    platform::{LocalMediaSnapshot, PlatformMediaAdapter, SpotifyProcessMonitor},
    sessions::SessionTracker,
    spotify::{PlaybackSnapshot, SpotifyClient},
    storage::Storage,
};

const VIEW_STATE_EVENT: &str = "view-state://changed";
const COMMAND_DEBOUNCE: Duration = Duration::from_millis(300);

pub struct PlaybackCoordinator {
    app: tauri::AppHandle,
    auth: Arc<AuthManager>,
    spotify: Arc<SpotifyClient>,
    local_media: Arc<dyn PlatformMediaAdapter>,
    storage: Storage,
    settings: Arc<RwLock<AppSettings>>,
    state: RwLock<ViewState>,
    sessions: Mutex<SessionTracker>,
    command_lock: Mutex<()>,
    last_command: Mutex<Option<Instant>>,
    rate_limit_until: Mutex<Option<Instant>>,
    context_cache: Mutex<HashMap<String, String>>,
    started: AtomicBool,
}

impl PlaybackCoordinator {
    pub fn new(
        app: tauri::AppHandle,
        auth: Arc<AuthManager>,
        spotify: Arc<SpotifyClient>,
        local_media: Arc<dyn PlatformMediaAdapter>,
        storage: Storage,
        settings: Arc<RwLock<AppSettings>>,
    ) -> Self {
        let recovered = storage.load_session();
        Self {
            app,
            auth,
            spotify,
            local_media,
            storage,
            settings,
            state: RwLock::new(ViewState::default()),
            sessions: Mutex::new(SessionTracker::new(recovered)),
            command_lock: Mutex::new(()),
            last_command: Mutex::new(None),
            rate_limit_until: Mutex::new(None),
            context_cache: Mutex::new(HashMap::new()),
            started: AtomicBool::new(false),
        }
    }

    pub async fn initialize_state(&self) {
        let settings = self.settings.read().await;
        self.mutate_state(|state| {
            state.settings = (&*settings).into();
            if !self.auth.is_configured() {
                state.availability = Availability::ConfigurationRequired;
                state.status_message = Some("Build with a Spotify client ID to connect.".into());
                state.status_tone = StatusTone::Warning;
            }
        })
        .await;
    }

    pub fn start(self: &Arc<Self>) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        let process_coordinator = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            process_coordinator.process_loop().await;
        });
        let playback_coordinator = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            playback_coordinator.playback_loop().await;
        });
    }

    pub async fn view_state(&self) -> ViewState {
        self.state.read().await.clone()
    }

    pub async fn begin_authorization(&self) -> AppResult<()> {
        self.mutate_state(|state| {
            state.status_message = Some("Waiting for Spotify authorization…".into());
            state.status_tone = StatusTone::Neutral;
        })
        .await;
        match self.auth.begin_authorization().await {
            Ok(()) => {
                self.refresh_playback(true).await;
                Ok(())
            }
            Err(error) => {
                self.handle_error(&error, None).await;
                Err(error)
            }
        }
    }

    pub async fn reconnect(&self) -> AppResult<()> {
        self.auth.invalidate_access_token().await;
        if self.auth.has_credentials().await? {
            self.refresh_playback(true).await;
            Ok(())
        } else {
            self.begin_authorization().await
        }
    }

    pub async fn transport(&self, command: CommandName) -> AppResult<()> {
        let _command_guard = self
            .command_lock
            .try_lock()
            .map_err(|_| AppError::SpotifyApi("another playback command is pending".into()))?;
        {
            let mut last = self.last_command.lock().await;
            if last.is_some_and(|instant| instant.elapsed() < COMMAND_DEBOUNCE) {
                return Ok(());
            }
            *last = Some(Instant::now());
        }
        self.mutate_state(|state| {
            state.command.pending = Some(command);
            state.command.last_error = None;
        })
        .await;

        let is_playing = self.state.read().await.playback.is_playing.unwrap_or(false);
        let prefer_local = self
            .settings
            .read()
            .await
            .playback
            .prefer_local_windows_controls;
        let local_result = if prefer_local {
            self.local_media.transport(command).await
        } else {
            Err(AppError::Platform("local controls disabled".into()))
        };
        let result = if local_result.as_ref().is_ok_and(|accepted| *accepted) {
            Ok(())
        } else {
            self.spotify.transport(command, is_playing).await
        };

        match result {
            Ok(()) => {
                if command == CommandName::TogglePlayPause {
                    self.mutate_state(|state| {
                        state.playback.is_playing = Some(!is_playing);
                    })
                    .await;
                }
                tokio::time::sleep(Duration::from_millis(900)).await;
                self.refresh_playback(true).await;
                self.mutate_state(|state| state.command.pending = None)
                    .await;
                Ok(())
            }
            Err(error) => {
                self.mutate_state(|state| {
                    state.command.pending = None;
                    state.command.last_error = Some(error.to_string());
                })
                .await;
                self.handle_error(&error, None).await;
                Err(error)
            }
        }
    }

    pub async fn settings_changed(&self, settings: &AppSettings) {
        self.mutate_state(|state| state.settings = settings.into())
            .await;
    }

    pub async fn checkpoint(&self) -> AppResult<()> {
        let mut sessions = self.sessions.lock().await;
        sessions.tick();
        if let Some(checkpoint) = sessions.checkpoint() {
            self.storage.save_session(&checkpoint)?;
        }
        Ok(())
    }

    async fn process_loop(self: Arc<Self>) {
        let mut monitor = SpotifyProcessMonitor::new();
        let mut checkpoint_interval = tokio::time::interval(Duration::from_secs(30));
        checkpoint_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            let observation = monitor.observe();
            let snapshot = {
                let mut sessions = self.sessions.lock().await;
                sessions.observe_process(observation.running, observation.identity);
                sessions.snapshot()
            };
            if !snapshot.spotify_running
                && let Err(error) = self.storage.clear_session()
            {
                tracing::warn!(%error, "stale session checkpoint cleanup failed");
            }
            self.mutate_state(|state| state.session = snapshot).await;
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                _ = checkpoint_interval.tick() => {
                    if let Err(error) = self.checkpoint().await {
                        tracing::warn!(%error, "session checkpoint failed");
                    }
                }
            }
        }
    }

    async fn playback_loop(self: Arc<Self>) {
        loop {
            let visible = self
                .app
                .get_webview_window("main")
                .and_then(|window| window.is_visible().ok())
                .unwrap_or(false);
            let running = self.state.read().await.session.spotify_running;
            if running {
                self.refresh_playback(false).await;
            } else {
                self.mutate_state(|state| {
                    state.availability = Availability::SpotifyClosed;
                    state.playback.can_control = false;
                    state.status_message = None;
                    state.status_tone = StatusTone::Neutral;
                })
                .await;
            }
            let delay = if !running {
                Duration::from_secs(5)
            } else if visible && self.state.read().await.playback.is_playing == Some(true) {
                Duration::from_secs(12)
            } else if visible {
                Duration::from_secs(30)
            } else {
                Duration::from_secs(60)
            };
            tokio::time::sleep(delay).await;
        }
    }

    async fn refresh_playback(&self, force: bool) {
        if !force
            && self
                .rate_limit_until
                .lock()
                .await
                .is_some_and(|until| until > Instant::now())
        {
            return;
        }
        if !self.auth.is_configured() {
            self.mutate_state(|state| {
                state.availability = Availability::ConfigurationRequired;
                state.playback.can_control = false;
            })
            .await;
            return;
        }
        match self.auth.has_credentials().await {
            Ok(false) => {
                self.mutate_state(|state| {
                    state.availability = Availability::AuthenticationRequired;
                    state.playback.can_control = false;
                    state.status_message = None;
                })
                .await;
                return;
            }
            Err(error) => {
                self.handle_error(&error, None).await;
                return;
            }
            Ok(true) => {}
        }

        let local = self.local_media.snapshot().await.ok().flatten();
        match self.spotify.get_playback().await {
            Ok(Some(mut snapshot)) => {
                self.resolve_context(&mut snapshot).await;
                let previous_uri = self
                    .state
                    .read()
                    .await
                    .playback
                    .item
                    .as_ref()
                    .and_then(|item| item.provider_uri.clone());
                let current_uri = snapshot
                    .playback
                    .item
                    .as_ref()
                    .and_then(|item| item.provider_uri.clone());
                merge_local_snapshot(&mut snapshot, local.as_ref());
                let queue = if force || previous_uri != current_uri {
                    match self.spotify.get_queue().await {
                        Ok(queue) => Some(queue),
                        Err(error) => {
                            tracing::warn!(%error, "queue refresh failed");
                            None
                        }
                    }
                } else {
                    None
                };
                let confirmed_playing = snapshot.playback.is_playing == Some(true);
                self.sessions
                    .lock()
                    .await
                    .set_confirmed_playing(confirmed_playing);
                self.mutate_state(|state| {
                    state.availability = if snapshot.playback.item.is_some() {
                        Availability::Ready
                    } else {
                        Availability::NoActiveDevice
                    };
                    state.playback = snapshot.playback;
                    if let Some(queue) = queue {
                        state.queue = queue;
                    }
                    state.status_message = None;
                    state.status_tone = StatusTone::Positive;
                })
                .await;
            }
            Ok(None) => {
                self.sessions.lock().await.set_confirmed_playing(false);
                self.mutate_state(|state| {
                    state.availability = Availability::NoActiveDevice;
                    state.playback = Default::default();
                    state.queue = Default::default();
                    state.status_message = None;
                    state.status_tone = StatusTone::Neutral;
                })
                .await;
            }
            Err(error) => self.handle_error(&error, local.as_ref()).await,
        }
    }

    async fn resolve_context(&self, snapshot: &mut PlaybackSnapshot) {
        let Some(href) = snapshot.context_href.as_deref() else {
            return;
        };
        if let Some(label) = self.context_cache.lock().await.get(href).cloned() {
            snapshot.playback.context_label = Some(label);
            return;
        }
        if let Ok(Some(label)) = self.spotify.resolve_context_label(href).await {
            self.context_cache
                .lock()
                .await
                .insert(href.to_owned(), label.clone());
            snapshot.playback.context_label = Some(label);
        }
    }

    async fn handle_error(&self, error: &AppError, local: Option<&LocalMediaSnapshot>) {
        if let AppError::RateLimited(retry_after) = error {
            *self.rate_limit_until.lock().await = Some(Instant::now() + *retry_after);
        }
        self.sessions.lock().await.set_confirmed_playing(false);
        self.mutate_state(|state| {
            state.availability = match error {
                AppError::ConfigurationRequired => Availability::ConfigurationRequired,
                AppError::AuthenticationRequired => Availability::AuthenticationRequired,
                AppError::NoActiveDevice => Availability::NoActiveDevice,
                AppError::RateLimited(_) => Availability::RateLimited,
                AppError::NetworkUnavailable => Availability::Offline,
                _ => Availability::Offline,
            };
            state.status_tone = match error {
                AppError::NetworkUnavailable | AppError::RateLimited(_) => StatusTone::Warning,
                _ => StatusTone::Critical,
            };
            state.status_message = Some(error.to_string());
            state.queue.availability = if state.queue.next.is_some() {
                QueueAvailability::Stale
            } else {
                QueueAvailability::Unavailable
            };
            state.playback.freshness = if state.playback.item.is_some() {
                Freshness::Stale
            } else {
                Freshness::Unknown
            };
            state.playback.can_control = false;
            if let Some(local) = local {
                apply_local_to_state(state, local);
            }
        })
        .await;
    }

    async fn mutate_state(&self, mutate: impl FnOnce(&mut ViewState)) {
        let snapshot = {
            let mut state = self.state.write().await;
            mutate(&mut state);
            state.revision = state.revision.wrapping_add(1);
            state.clone()
        };
        if let Err(error) = self.app.emit(VIEW_STATE_EVENT, snapshot) {
            tracing::warn!(%error, "failed to publish view state");
        }
    }
}

fn merge_local_snapshot(snapshot: &mut PlaybackSnapshot, local: Option<&LocalMediaSnapshot>) {
    let Some(local) = local else {
        return;
    };
    if snapshot.playback.item.is_none() {
        snapshot.playback.item = Some(local_media_item(local));
    }
    snapshot.playback.is_playing = Some(local.is_playing);
    snapshot.playback.progress_ms = local.progress_ms.or(snapshot.playback.progress_ms);
    snapshot.playback.can_control = true;
    snapshot.playback.observed_at = Utc::now();
}

fn apply_local_to_state(state: &mut ViewState, local: &LocalMediaSnapshot) {
    state.playback.item = Some(local_media_item(local));
    state.playback.is_playing = Some(local.is_playing);
    state.playback.progress_ms = local.progress_ms;
    state.playback.freshness = Freshness::Fresh;
    state.playback.can_control = true;
    state.playback.observed_at = Utc::now();
}

fn local_media_item(local: &LocalMediaSnapshot) -> MediaItem {
    MediaItem {
        provider_uri: None,
        kind: MediaKind::Unknown,
        title: local.title.clone(),
        creators: if local.artist.is_empty() {
            Vec::new()
        } else {
            vec![local.artist.clone()]
        },
        album_or_show: local.album.clone(),
        duration_ms: local.duration_ms,
        artwork_url: None,
        is_local: false,
        is_explicit: None,
    }
}
