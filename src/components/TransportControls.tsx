import type { CommandName } from "../types";
import { NextIcon, PauseIcon, PlayIcon, PreviousIcon } from "./Icons";
import { TransportButton } from "./audio-ui/TransportButton";

export interface TransportControlsProps {
  isPlaying: boolean;
  canControl: boolean;
  pending: CommandName | null;
  onCommand: (command: CommandName) => void;
}

export function TransportControls({ isPlaying, canControl, pending, onCommand }: TransportControlsProps) {
  const disabled = !canControl || pending !== null;

  return (
    <div className="transport-controls" aria-label="Playback controls">
      <TransportButton
        label="Previous track"
        disabled={disabled}
        pending={pending === "previous"}
        onPress={() => onCommand("previous")}
      >
        <PreviousIcon />
      </TransportButton>
      <TransportButton
        label={isPlaying ? "Pause" : "Play"}
        active={isPlaying}
        primary
        disabled={disabled}
        pending={pending === "togglePlayPause"}
        onPress={() => onCommand("togglePlayPause")}
      >
        {isPlaying ? <PauseIcon /> : <PlayIcon />}
      </TransportButton>
      <TransportButton
        label="Next track"
        disabled={disabled}
        pending={pending === "next"}
        onPress={() => onCommand("next")}
      >
        <NextIcon />
      </TransportButton>
    </div>
  );
}
