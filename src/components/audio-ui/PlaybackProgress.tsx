/*
 * Adapted from AudioUI LinearStrip and ValueStrip.
 * Copyright (c) 2026 Tylium.
 * Copyright (c) 2026 Vladislav Kondratyev (widget-specific modifications).
 * SPDX-License-Identifier: GPL-3.0-only
 */

export interface PlaybackProgressProps {
  progressMs: number | null;
  durationMs: number | null;
  stale?: boolean;
}

export function PlaybackProgress({ progressMs, durationMs, stale = false }: PlaybackProgressProps) {
  const hasDuration = durationMs !== null && durationMs > 0;
  const normalized = hasDuration ? Math.max(0, Math.min(1, (progressMs ?? 0) / durationMs)) : 0;
  const valueNow = Math.round(normalized * 100);

  return (
    <div
      className="playback-progress"
      role="progressbar"
      aria-label="Track progress"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={hasDuration ? valueNow : undefined}
      aria-valuetext={hasDuration ? `${valueNow}%` : "Progress unavailable"}
      data-stale={stale || undefined}
    >
      <svg viewBox="0 0 100 4" preserveAspectRatio="none" aria-hidden="true">
        <rect className="playback-progress__track" x="0" y="0.5" width="100" height="3" rx="1.5" />
        {normalized > 0 && (
          <rect
            className="playback-progress__value"
            x="0"
            y="0.5"
            width={normalized * 100}
            height="3"
            rx="1.5"
          />
        )}
      </svg>
    </div>
  );
}
