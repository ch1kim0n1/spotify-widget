# Architecture

Spotify Companion Widget is a Tauri 2 desktop process with a presentation-only React webview and a
serialized Rust playback coordinator.

## Data flow

1. The process monitor identifies the native Spotify process and its process-session identity.
2. The coordinator reads account playback from the Spotify Web API and optional local metadata from
   Windows GSMTC.
3. Queue, context, process state, command state, and monotonic session counters merge into one
   immutable `ViewState`.
4. Rust publishes `view-state://changed`; React renders the latest snapshot and interpolates only the
   visible progress indicator.
5. React emits narrow transport, authentication, window, and settings commands. No token, URL, file,
   or shell primitive crosses the webview boundary.

## Ownership

- `src/`: rendering, accessibility, progress interpolation, and typed command bindings.
- `src-tauri/src/auth/`: PKCE S256, loopback callback validation, token refresh, and native secret
  storage.
- `src-tauri/src/spotify/`: tolerant Spotify payload parsing, endpoint allowlisting, error taxonomy,
  rate limits, and bounded retries.
- `src-tauri/src/playback/`: source reconciliation, command serialization, queue invalidation, and
  state publication.
- `src-tauri/src/platform/`: Spotify process observation, Windows GSMTC, and macOS panel behavior.
- `src-tauri/src/sessions/`: monotonic process/listening counters and suspend-sized gap exclusion.
- `src-tauri/src/storage/`: versioned settings and atomic session checkpoints.
- `src-tauri/src/app/`: tray/menu lifecycle, window policy, monitor recovery, and redacted logging.

## Polling policy

The coordinator uses event-driven UI updates and bounded background polling:

- Spotify closed: process observation only, every 5 seconds.
- Visible and playing: playback refresh every 12 seconds.
- Visible and paused: every 30 seconds.
- Hidden: every 60 seconds.
- A `Retry-After` response suspends Spotify requests for the server-defined interval.

Windows GSMTC supplies low-latency local control and metadata when available. The Web API remains the
authority for account playback, queue, context, and cross-device control.

## Persistence

Only settings, redacted logs, and session checkpoints are stored as local files. Refresh credentials
use Windows Credential Manager or macOS Keychain through `keyring`. Access tokens remain in memory.
Files use same-directory atomic replacement. A corrupt settings file receives a timestamped backup
before defaults load.

Window coordinates are persisted in logical pixels and clamped against current physical monitor
bounds after DPI conversion. At least 24 pixels remain reachable.

## Platform shell

- Windows: hidden taskbar/Alt+Tab presence, topmost tool-style Tauri window, tray icon, GSMTC, and
  Credential Manager.
- macOS: accessory activation policy, menu-bar status item, `NSPanel`, floating level, all Spaces,
  full-screen auxiliary behavior, Keychain, and LaunchAgent login item.

Close shortcuts and the close control hide the widget. Only the explicit tray Kill/Quit command
terminates the process.
