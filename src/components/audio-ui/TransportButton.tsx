/*
 * Adapted from AudioUI ButtonView and BooleanControl.
 * Copyright (c) 2026 Tylium.
 * Copyright (c) 2026 Vladislav Kondratyev (widget-specific modifications).
 * SPDX-License-Identifier: GPL-3.0-only
 */

import type { ReactNode } from "react";

export interface TransportButtonProps {
  label: string;
  active?: boolean;
  primary?: boolean;
  disabled?: boolean;
  pending?: boolean;
  children: ReactNode;
  onPress: () => void;
}

export function TransportButton({
  label,
  active = false,
  primary = false,
  disabled = false,
  pending = false,
  children,
  onPress,
}: TransportButtonProps) {
  return (
    <button
      type="button"
      className="transport-button"
      data-active={active || undefined}
      data-primary={primary || undefined}
      data-pending={pending || undefined}
      disabled={disabled}
      aria-label={label}
      aria-pressed={active}
      aria-busy={pending}
      onClick={onPress}
      data-tauri-drag-region="false"
    >
      <svg viewBox="0 0 48 48" role="presentation">
        <rect className="transport-button__shadow" x="3" y="4" width="42" height="42" rx="14" />
        <rect className="transport-button__surface" x="3" y="2" width="42" height="42" rx="14" />
        <rect className="transport-button__edge" x="3.75" y="2.75" width="40.5" height="40.5" rx="13.25" />
      </svg>
      <span className="transport-button__icon">{children}</span>
      {pending && <span className="transport-button__pending" aria-hidden="true" />}
    </button>
  );
}
