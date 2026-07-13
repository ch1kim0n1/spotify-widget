# Third-party notices

Spotify Companion Widget is licensed under GPL-3.0-only. Dependency license metadata is available
from the JavaScript and Rust lockfiles.

## AudioUI-derived presentation patterns

The following files adapt selected implementation patterns from the AudioUI project by Tylium:

- `src/components/audio-ui/TransportButton.tsx`
- `src/components/audio-ui/PlaybackProgress.tsx`
- `src/styles/audio-ui.css`

The adaptations cover SVG control surfaces, linear value strips, CSS-variable theming, focus
highlighting, and adaptive sizing. The widget-specific implementations do not package the AudioUI
monorepo.

Copyright (c) 2026 Tylium.

AudioUI is available under GPL-3.0-only or the Tylium Evolutive License Framework. This project
uses the GPL-3.0-only option. See `LICENSE`.

## Spotify

Spotify is a trademark of Spotify AB. Spotify Companion Widget is an independent project and is not
affiliated with, endorsed by, or sponsored by Spotify AB. Album artwork and playback metadata remain
the property of their respective owners and are displayed transiently from Spotify API responses.

## Tauri and other dependencies

Tauri, React, Tokio, reqwest, keyring, and the remaining dependencies retain their respective
licenses and copyright notices. Release CI must pass the dependency audit before publication.
