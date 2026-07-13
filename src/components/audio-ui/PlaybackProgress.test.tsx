import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PlaybackProgress } from "./PlaybackProgress";

describe("PlaybackProgress", () => {
  it("exposes clamped progress to assistive technology", () => {
    render(<PlaybackProgress progressMs={150_000} durationMs={100_000} />);
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "100");
  });

  it("describes unavailable progress without fabricating a value", () => {
    render(<PlaybackProgress progressMs={null} durationMs={null} />);
    const progress = screen.getByRole("progressbar");
    expect(progress).not.toHaveAttribute("aria-valuenow");
    expect(progress).toHaveAttribute("aria-valuetext", "Progress unavailable");
  });
});
