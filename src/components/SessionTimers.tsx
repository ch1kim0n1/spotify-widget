import type { SessionView, SettingsView } from "../types";

function formatDuration(milliseconds: number): string {
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  return [hours, minutes, seconds].map((value) => String(value).padStart(2, "0")).join(":");
}

export interface SessionTimersProps {
  session: SessionView;
  settings: SettingsView;
}

export function SessionTimers({ session, settings }: SessionTimersProps) {
  if (!settings.showSpotifyOpenTime && !settings.showListeningTime) return null;

  return (
    <div className="session-timers" aria-label="Spotify session timers">
      {settings.showSpotifyOpenTime && (
        <span title="Time Spotify has been open in this process session">
          <span className="session-timers__label">Open</span>
          <time>{formatDuration(session.spotifyOpenMs)}</time>
        </span>
      )}
      {settings.showListeningTime && (
        <span title="Confirmed account playback time in this process session">
          <span className="session-timers__label">Listen</span>
          <time>{formatDuration(session.activeListeningMs)}</time>
        </span>
      )}
    </div>
  );
}
