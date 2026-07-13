use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{domain::SessionView, storage::SessionCheckpoint};

const PROCESS_EXIT_DEBOUNCE: Duration = Duration::from_secs(3);
const MAX_UNVERIFIED_DELTA: Duration = Duration::from_secs(8);

#[derive(Debug)]
pub struct SessionTracker {
    session_id: Uuid,
    process_identity: Option<String>,
    started_at: DateTime<Utc>,
    spotify_open_ms: u64,
    active_listening_ms: u64,
    running: bool,
    confirmed_playing: bool,
    sleeping: bool,
    last_tick: Instant,
    absent_since: Option<Instant>,
    recovered: Option<SessionCheckpoint>,
}

impl SessionTracker {
    pub fn new(recovered: Option<SessionCheckpoint>) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            process_identity: None,
            started_at: Utc::now(),
            spotify_open_ms: 0,
            active_listening_ms: 0,
            running: false,
            confirmed_playing: false,
            sleeping: false,
            last_tick: Instant::now(),
            absent_since: None,
            recovered,
        }
    }

    pub fn observe_process(&mut self, present: bool, identity: Option<String>) {
        self.observe_process_at(present, identity, Instant::now(), Utc::now());
    }

    pub fn observe_process_at(
        &mut self,
        present: bool,
        identity: Option<String>,
        now: Instant,
        wall_now: DateTime<Utc>,
    ) {
        self.accumulate(now);
        if present {
            self.absent_since = None;
            let identity = identity.unwrap_or_else(|| "spotify-process".into());
            if !self.running || self.process_identity.as_deref() != Some(identity.as_str()) {
                self.start_session(identity, wall_now, now);
            }
            self.running = true;
        } else if self.running {
            let absent_since = self.absent_since.get_or_insert(now);
            if now.duration_since(*absent_since) >= PROCESS_EXIT_DEBOUNCE {
                self.end_session(wall_now);
            }
        }
    }

    pub fn set_confirmed_playing(&mut self, playing: bool) {
        self.accumulate(Instant::now());
        self.confirmed_playing = playing;
    }

    pub fn tick(&mut self) {
        self.accumulate(Instant::now());
    }

    #[allow(dead_code)]
    pub fn on_sleep(&mut self) {
        self.accumulate(Instant::now());
        self.sleeping = true;
    }

    #[allow(dead_code)]
    pub fn on_resume(&mut self) {
        self.sleeping = false;
        self.last_tick = Instant::now();
    }

    pub fn snapshot(&self) -> SessionView {
        SessionView {
            spotify_open_ms: self.spotify_open_ms,
            active_listening_ms: self.active_listening_ms,
            spotify_running: self.running,
        }
    }

    pub fn checkpoint(&self) -> Option<SessionCheckpoint> {
        let identity = self.process_identity.clone()?;
        Some(SessionCheckpoint {
            schema_version: 1,
            session_id: self.session_id,
            platform_process_identity: identity,
            started_at: self.started_at,
            spotify_open_ms: self.spotify_open_ms,
            active_listening_ms: self.active_listening_ms,
            last_checkpoint_at: Utc::now(),
            was_playing: self.confirmed_playing,
        })
    }

    fn start_session(&mut self, identity: String, wall_now: DateTime<Utc>, now: Instant) {
        let recovered = self
            .recovered
            .take()
            .filter(|checkpoint| checkpoint.platform_process_identity == identity);
        if let Some(checkpoint) = recovered {
            self.session_id = checkpoint.session_id;
            self.started_at = checkpoint.started_at;
            self.spotify_open_ms = checkpoint.spotify_open_ms;
            self.active_listening_ms = checkpoint.active_listening_ms;
            self.confirmed_playing = false;
        } else {
            self.session_id = Uuid::new_v4();
            self.started_at = wall_now;
            self.spotify_open_ms = 0;
            self.active_listening_ms = 0;
            self.confirmed_playing = false;
        }
        self.process_identity = Some(identity);
        self.last_tick = now;
    }

    fn end_session(&mut self, wall_now: DateTime<Utc>) {
        self.running = false;
        self.confirmed_playing = false;
        self.process_identity = None;
        self.absent_since = None;
        self.session_id = Uuid::new_v4();
        self.started_at = wall_now;
        self.spotify_open_ms = 0;
        self.active_listening_ms = 0;
    }

    fn accumulate(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        if self.sleeping || !self.running || elapsed > MAX_UNVERIFIED_DELTA {
            return;
        }
        let delta_ms = elapsed.as_millis().min(u128::from(u64::MAX)) as u64;
        self.spotify_open_ms = self.spotify_open_ms.saturating_add(delta_ms);
        if self.confirmed_playing {
            self.active_listening_ms = self.active_listening_ms.saturating_add(delta_ms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_open_and_listening_separately() {
        let base = Instant::now();
        let mut tracker = SessionTracker::new(None);
        tracker.observe_process_at(true, Some("spotify:1".into()), base, Utc::now());
        tracker.confirmed_playing = true;
        tracker.accumulate(base + Duration::from_secs(5));
        tracker.confirmed_playing = false;
        tracker.accumulate(base + Duration::from_secs(8));
        assert_eq!(tracker.spotify_open_ms, 8_000);
        assert_eq!(tracker.active_listening_ms, 5_000);
    }

    #[test]
    fn ignores_sleep_sized_unverified_delta() {
        let base = Instant::now();
        let mut tracker = SessionTracker::new(None);
        tracker.observe_process_at(true, Some("spotify:1".into()), base, Utc::now());
        tracker.confirmed_playing = true;
        tracker.accumulate(base + Duration::from_secs(60));
        assert_eq!(tracker.spotify_open_ms, 0);
        assert_eq!(tracker.active_listening_ms, 0);
    }

    #[test]
    fn debounces_short_process_absence() {
        let base = Instant::now();
        let mut tracker = SessionTracker::new(None);
        tracker.observe_process_at(true, Some("spotify:1".into()), base, Utc::now());
        tracker.observe_process_at(false, None, base + Duration::from_secs(1), Utc::now());
        tracker.observe_process_at(
            true,
            Some("spotify:1".into()),
            base + Duration::from_secs(2),
            Utc::now(),
        );
        assert!(tracker.snapshot().spotify_running);
    }
}
