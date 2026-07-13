import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { axe } from "vitest-axe";
import type { ViewState } from "./types";

const bridge = vi.hoisted(() => ({
  getViewState: vi.fn<() => Promise<ViewState>>(),
  subscribeToViewState: vi.fn(() => Promise.resolve(() => undefined)),
  runTransportCommand: vi.fn(() => Promise.resolve()),
  beginSpotifyAuth: vi.fn(() => Promise.resolve()),
  reconnectSpotify: vi.fn(() => Promise.resolve()),
  hideWidget: vi.fn(() => Promise.resolve()),
  updateSettings: vi.fn(() => Promise.resolve()),
}));

vi.mock("./lib/bridge", () => bridge);

import App from "./App";

const READY_STATE: ViewState = {
  availability: "ready",
  playback: {
    item: {
      providerUri: "spotify:track:1",
      kind: "track",
      title: "Signal Path",
      creators: ["Test Artist"],
      albumOrShow: "Test Album",
      durationMs: 180_000,
      artworkUrl: null,
      isLocal: false,
      isExplicit: false,
    },
    isPlaying: false,
    canControl: true,
    progressMs: 30_000,
    observedAt: new Date().toISOString(),
    freshness: "fresh",
    contextLabel: "Focus Mix",
    deviceName: "Office",
  },
  queue: {
    next: {
      providerUri: "spotify:track:2",
      kind: "track",
      title: "Next Signal",
      creators: ["Second Artist"],
      albumOrShow: null,
      durationMs: 160_000,
      artworkUrl: null,
      isLocal: false,
      isExplicit: null,
    },
    availability: "available",
  },
  session: {
    spotifyOpenMs: 3_661_000,
    activeListeningMs: 125_000,
    spotifyRunning: true,
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
  statusMessage: null,
  statusTone: "positive",
  revision: 1,
};

describe("Spotify Companion Widget", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    bridge.getViewState.mockResolvedValue(READY_STATE);
  });

  it("renders current playback, context, queue, and timers", async () => {
    render(<App />);
    expect(await screen.findByRole("heading", { name: "Signal Path" })).toBeInTheDocument();
    expect(screen.getByText("Test Artist")).toBeInTheDocument();
    expect(screen.getByText("Focus Mix")).toBeInTheDocument();
    expect(screen.getByText(/Next Signal/)).toBeInTheDocument();
    expect(screen.getByText("01:01:01")).toBeInTheDocument();
    expect(screen.getByText("00:02:05")).toBeInTheDocument();
  });

  it("routes transport actions through the narrow bridge", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: "Signal Path" });
    await user.click(screen.getByRole("button", { name: "Play" }));
    expect(bridge.runTransportCommand).toHaveBeenCalledWith("togglePlayPause");
  });

  it("shows a connect action when authorization is required", async () => {
    bridge.getViewState.mockResolvedValue({
      ...READY_STATE,
      availability: "authenticationRequired",
      playback: { ...READY_STATE.playback, item: null, canControl: false },
    });
    const user = userEvent.setup();
    render(<App />);
    const connect = await screen.findByRole("button", { name: "Connect" });
    await user.click(connect);
    expect(bridge.beginSpotifyAuth).toHaveBeenCalledOnce();
  });

  it("hides on Escape without terminating the process", async () => {
    render(<App />);
    await screen.findByRole("heading", { name: "Signal Path" });
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await waitFor(() => expect(bridge.hideWidget).toHaveBeenCalledOnce());
  });

  it("has no automated accessibility violations in the playback state", async () => {
    const { container } = render(<App />);
    await screen.findByRole("heading", { name: "Signal Path" });
    const results = await axe(container, {
      rules: { "color-contrast": { enabled: false } },
    });
    expect(results.violations).toEqual([]);
  });
});
