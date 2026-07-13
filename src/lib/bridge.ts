import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { CommandName, SettingsView, ViewState } from "../types";
import { EMPTY_VIEW_STATE } from "../types";

const VIEW_STATE_EVENT = "view-state://changed";
const DEVELOPMENT_VIEW_STATE: ViewState = {
  ...EMPTY_VIEW_STATE,
  availability: "ready",
  playback: {
    item: {
      providerUri: "spotify:track:preview",
      kind: "track",
      title: "Signal Path",
      creators: ["Companion Session"],
      albumOrShow: "Desktop Mix",
      durationMs: 224_000,
      artworkUrl: null,
      isLocal: false,
      isExplicit: false,
    },
    isPlaying: true,
    canControl: true,
    progressMs: 68_000,
    observedAt: new Date().toISOString(),
    freshness: "fresh",
    contextLabel: "Desktop Mix",
    deviceName: "This computer",
  },
  queue: {
    next: {
      providerUri: "spotify:track:preview-next",
      kind: "track",
      title: "Quiet Current",
      creators: ["Companion Session"],
      albumOrShow: "Desktop Mix",
      durationMs: 188_000,
      artworkUrl: null,
      isLocal: false,
      isExplicit: false,
    },
    availability: "available",
  },
  session: {
    spotifyOpenMs: 5_462_000,
    activeListeningMs: 3_925_000,
    spotifyRunning: true,
  },
  statusMessage: null,
  statusTone: "positive",
  revision: 1,
};

function isTauriRuntime(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

export async function getViewState(): Promise<ViewState> {
  if (!isTauriRuntime()) {
    return import.meta.env.DEV ? DEVELOPMENT_VIEW_STATE : EMPTY_VIEW_STATE;
  }
  return invoke<ViewState>("get_view_state");
}

export async function subscribeToViewState(onState: (state: ViewState) => void): Promise<UnlistenFn> {
  if (!isTauriRuntime()) {
    return () => undefined;
  }
  return listen<ViewState>(VIEW_STATE_EVENT, (event) => onState(event.payload));
}

export async function runTransportCommand(command: CommandName): Promise<void> {
  const commandName: Record<CommandName, string> = {
    previous: "transport_previous",
    togglePlayPause: "transport_toggle_play_pause",
    next: "transport_next",
  };
  await invoke(commandName[command]);
}

export async function beginSpotifyAuth(): Promise<void> {
  await invoke("begin_spotify_auth");
}

export async function reconnectSpotify(): Promise<void> {
  await invoke("reconnect_spotify");
}

export async function hideWidget(): Promise<void> {
  await invoke("hide_widget");
}

export async function updateSettings(settings: Partial<SettingsView>): Promise<void> {
  await invoke("update_settings", { patch: settings });
}
