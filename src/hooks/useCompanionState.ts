import { useCallback, useEffect, useMemo, useState } from "react";
import {
  beginSpotifyAuth,
  getViewState,
  hideWidget,
  reconnectSpotify,
  runTransportCommand,
  subscribeToViewState,
} from "../lib/bridge";
import type { CommandName, ViewState } from "../types";
import { EMPTY_VIEW_STATE } from "../types";

function interpolateProgress(state: ViewState, now: number): number | null {
  const progress = state.playback.progressMs;
  if (progress === null) return null;
  if (!state.playback.isPlaying || state.playback.freshness !== "fresh") return progress;

  const observedAt = Date.parse(state.playback.observedAt);
  const elapsed = Number.isFinite(observedAt) ? Math.max(0, now - observedAt) : 0;
  const duration = state.playback.item?.durationMs ?? Number.POSITIVE_INFINITY;
  return Math.min(progress + elapsed, duration);
}

export interface CompanionActions {
  transport: (command: CommandName) => Promise<void>;
  connect: () => Promise<void>;
  reconnect: () => Promise<void>;
  hide: () => Promise<void>;
}

export function useCompanionState(): {
  state: ViewState;
  progressMs: number | null;
  loading: boolean;
  actions: CompanionActions;
} {
  const [state, setState] = useState<ViewState>(EMPTY_VIEW_STATE);
  const [loading, setLoading] = useState(true);
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    let mounted = true;
    let unsubscribe: () => void = () => undefined;

    void getViewState()
      .then((initialState) => {
        if (mounted) setState(initialState);
      })
      .catch((error: unknown) => {
        if (!mounted) return;
        setState((current) => ({
          ...current,
          statusMessage: error instanceof Error ? error.message : "Unable to load companion state.",
          statusTone: "critical",
        }));
      })
      .finally(() => {
        if (mounted) setLoading(false);
      });

    void subscribeToViewState((nextState) => {
      if (mounted) setState(nextState);
    }).then((unlisten) => {
      if (mounted) unsubscribe = unlisten;
      else unlisten();
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, []);

  useEffect(() => {
    if (!state.playback.isPlaying || state.playback.freshness !== "fresh") return undefined;
    const interval = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(interval);
  }, [state.playback.freshness, state.playback.isPlaying]);

  const runAction = useCallback(async (action: () => Promise<void>) => {
    try {
      await action();
    } catch (error: unknown) {
      setState((current) => ({
        ...current,
        statusMessage: error instanceof Error ? error.message : "The action could not be completed.",
        statusTone: "critical",
      }));
    }
  }, []);

  const actions = useMemo<CompanionActions>(
    () => ({
      transport: (command) => runAction(() => runTransportCommand(command)),
      connect: () => runAction(beginSpotifyAuth),
      reconnect: () => runAction(reconnectSpotify),
      hide: () => runAction(hideWidget),
    }),
    [runAction],
  );

  return {
    state,
    progressMs: interpolateProgress(state, now),
    loading,
    actions,
  };
}
