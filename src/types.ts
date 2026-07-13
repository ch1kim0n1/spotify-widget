export type Availability =
  | "unknown"
  | "configurationRequired"
  | "spotifyClosed"
  | "authenticationRequired"
  | "noActiveDevice"
  | "ready"
  | "rateLimited"
  | "offline";

export type Freshness = "fresh" | "stale" | "unknown";
export type CommandName = "previous" | "togglePlayPause" | "next";
export type StatusTone = "neutral" | "positive" | "warning" | "critical";

export interface MediaItem {
  providerUri: string | null;
  kind: "track" | "episode" | "advertisement" | "local" | "unknown";
  title: string;
  creators: string[];
  albumOrShow: string | null;
  durationMs: number | null;
  artworkUrl: string | null;
  isLocal: boolean;
  isExplicit: boolean | null;
}

export interface PlaybackView {
  item: MediaItem | null;
  isPlaying: boolean | null;
  canControl: boolean;
  progressMs: number | null;
  observedAt: string;
  freshness: Freshness;
  contextLabel: string | null;
  deviceName: string | null;
}

export interface QueueView {
  next: MediaItem | null;
  availability: "available" | "empty" | "unavailable" | "stale";
}

export interface SessionView {
  spotifyOpenMs: number;
  activeListeningMs: number;
  spotifyRunning: boolean;
}

export interface CommandView {
  pending: CommandName | null;
  lastError: string | null;
}

export interface SettingsView {
  alwaysOnTop: boolean;
  launchAtLogin: boolean;
  showSpotifyOpenTime: boolean;
  showListeningTime: boolean;
}

export interface ViewState {
  availability: Availability;
  playback: PlaybackView;
  queue: QueueView;
  session: SessionView;
  command: CommandView;
  settings: SettingsView;
  statusMessage: string | null;
  statusTone: StatusTone;
  revision: number;
}

export const EMPTY_VIEW_STATE: ViewState = {
  availability: "unknown",
  playback: {
    item: null,
    isPlaying: null,
    canControl: false,
    progressMs: null,
    observedAt: new Date(0).toISOString(),
    freshness: "unknown",
    contextLabel: null,
    deviceName: null,
  },
  queue: {
    next: null,
    availability: "unavailable",
  },
  session: {
    spotifyOpenMs: 0,
    activeListeningMs: 0,
    spotifyRunning: false,
  },
  command: {
    pending: null,
    lastError: null,
  },
  settings: {
    alwaysOnTop: true,
    launchAtLogin: false,
    showSpotifyOpenTime: true,
    showListeningTime: true,
  },
  statusMessage: "Starting companion…",
  statusTone: "neutral",
  revision: 0,
};
