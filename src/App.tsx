import { useEffect } from "react";
import { CloseIcon } from "./components/Icons";
import { ConnectionPrompt } from "./components/ConnectionPrompt";
import { MediaArtwork } from "./components/MediaArtwork";
import { SessionTimers } from "./components/SessionTimers";
import { TransportControls } from "./components/TransportControls";
import { PlaybackProgress } from "./components/audio-ui/PlaybackProgress";
import { useCompanionState } from "./hooks/useCompanionState";

function creatorsLabel(creators: string[]): string {
  return creators.length > 0 ? creators.join(", ") : "Unknown artist";
}

function formatShortTime(milliseconds: number | null): string {
  if (milliseconds === null) return "–:––";
  const totalSeconds = Math.max(0, Math.floor(milliseconds / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export default function App() {
  const { state, progressMs, loading, actions } = useCompanionState();
  const item = state.playback.item;
  const showPlayer = item !== null;
  const isStale = state.playback.freshness === "stale";

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") void actions.hide();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [actions]);

  return (
    <main
      className="widget"
      data-tone={state.statusTone}
      data-loading={loading || undefined}
      data-tauri-drag-region
    >
      <div className="widget__ambient" aria-hidden="true" />
      <header className="widget__header" data-tauri-drag-region>
        <div className="widget__identity" data-tauri-drag-region>
          <span className="widget__signal" aria-hidden="true" />
          <span>Companion</span>
          {isStale && <span className="widget__stale">Last known</span>}
        </div>
        <button
          type="button"
          className="widget__close"
          aria-label="Hide widget"
          title="Hide widget"
          onClick={() => void actions.hide()}
          data-tauri-drag-region="false"
        >
          <CloseIcon />
        </button>
      </header>

      {showPlayer ? (
        <>
          <section className="player" aria-label="Current Spotify playback">
            <MediaArtwork url={item.artworkUrl} title={item.title} stale={isStale} />
            <div className="player__body">
              <div className="player__copy">
                <h1 title={item.title}>{item.title}</h1>
                <p className="player__artist" title={creatorsLabel(item.creators)}>
                  {creatorsLabel(item.creators)}
                </p>
                <p className="player__context" title={state.playback.contextLabel ?? "Unknown context"}>
                  <span>Playing from</span>
                  {state.playback.contextLabel ?? "Unknown context"}
                </p>
              </div>
              <TransportControls
                isPlaying={state.playback.isPlaying === true}
                canControl={state.playback.canControl}
                pending={state.command.pending}
                onCommand={(command) => void actions.transport(command)}
              />
              <div className="player__timeline">
                <span>{formatShortTime(progressMs)}</span>
                <PlaybackProgress progressMs={progressMs} durationMs={item.durationMs} stale={isStale} />
                <span>{formatShortTime(item.durationMs)}</span>
              </div>
            </div>
          </section>

          <footer className="widget__footer" data-tauri-drag-region>
            <div className="next-item" title={state.queue.next?.title ?? "Next item unavailable"}>
              <span className="next-item__label">Next</span>
              <span className="next-item__title">
                {state.queue.next
                  ? `${state.queue.next.title} — ${creatorsLabel(state.queue.next.creators)}`
                  : state.queue.availability === "empty"
                    ? "End of queue"
                    : "Unavailable"}
              </span>
            </div>
            <SessionTimers session={state.session} settings={state.settings} />
          </footer>
        </>
      ) : (
        <ConnectionPrompt
          availability={state.availability === "ready" ? "unknown" : state.availability}
          onConnect={() => void actions.connect()}
          onReconnect={() => void actions.reconnect()}
        />
      )}

      {(state.statusMessage || state.command.lastError) && showPlayer && (
        <div className="widget__status" role="status" title={state.command.lastError ?? undefined}>
          {state.command.lastError ?? state.statusMessage}
        </div>
      )}
    </main>
  );
}
