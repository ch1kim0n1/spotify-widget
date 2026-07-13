import type { Availability } from "../types";

const CONTENT: Record<
  Exclude<Availability, "ready">,
  { title: string; detail: string; action?: "connect" | "reconnect" }
> = {
  unknown: {
    title: "Checking Spotify",
    detail: "Resolving the local app and account playback state.",
  },
  configurationRequired: {
    title: "Spotify client ID required",
    detail: "Build the app with SPOTIFY_CLIENT_ID to connect an account.",
  },
  spotifyClosed: {
    title: "Spotify is not running",
    detail: "Open Spotify to begin a desktop session.",
  },
  authenticationRequired: {
    title: "Connect Spotify",
    detail: "Authorization is required for playback context and queue.",
    action: "connect",
  },
  noActiveDevice: {
    title: "No active Spotify device",
    detail: "Start playback on this computer or another Spotify device.",
  },
  rateLimited: {
    title: "Refreshing shortly",
    detail: "Spotify temporarily limited requests.",
  },
  offline: {
    title: "Connection unavailable",
    detail: "Playback will refresh when the network returns.",
    action: "reconnect",
  },
};

export interface ConnectionPromptProps {
  availability: Exclude<Availability, "ready">;
  onConnect: () => void;
  onReconnect: () => void;
}

export function ConnectionPrompt({ availability, onConnect, onReconnect }: ConnectionPromptProps) {
  const content = CONTENT[availability];

  return (
    <section className="connection-prompt" aria-live="polite">
      <span className="connection-prompt__pulse" aria-hidden="true" />
      <div>
        <h1>{content.title}</h1>
        <p>{content.detail}</p>
      </div>
      {content.action && (
        <button
          type="button"
          className="connection-prompt__action"
          onClick={content.action === "connect" ? onConnect : onReconnect}
          data-tauri-drag-region="false"
        >
          {content.action === "connect" ? "Connect" : "Retry"}
        </button>
      )}
    </section>
  );
}
