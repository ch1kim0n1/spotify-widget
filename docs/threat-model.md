# Threat model

## Protected assets

- Spotify refresh and access credentials
- OAuth authorization code, verifier, and state
- Playback metadata and account activity
- Local settings, session counters, and logs
- Native command execution and filesystem access

## Trust boundaries

The React webview is untrusted presentation input. Rust owns credentials, network I/O, persistence,
timers, process inspection, and native window behavior. Spotify endpoints, artwork responses, browser
callbacks, settings files, and native media sessions are external inputs.

## Controls

### OAuth and credentials

- Authorization Code with PKCE S256; no client secret is embedded.
- State and verifier use fresh URL-safe randomness for every attempt.
- The callback binds only to `127.0.0.1` on a dynamic port, expires after three minutes, and accepts
  only `/callback` with exactly one matching state and one code.
- The registered dashboard URI is `http://127.0.0.1/callback`; Spotify permits the authorization
  request to add a dynamic port for a loopback IP literal.
- Refresh tokens use Windows Credential Manager or macOS Keychain. Access tokens remain in memory.
- Tokens and callback query strings never enter logs or React state.

### Webview

- Tauri capabilities permit core event listening and drag initiation only.
- No generic shell, filesystem, HTTP, process, or secret command is exposed.
- CSP restricts scripts and network connections to the packaged app and Tauri IPC. Images are limited
  to packaged/data assets and Spotify CDN hosts.
- Artwork URLs require HTTPS and an allowlisted `scdn.co` host before entering `ViewState`.

### Network and parsing

- Spotify uses Rustls and a fixed `https://api.spotify.com/v1` base.
- Context hyperlinks require HTTPS, the exact `api.spotify.com` host, and a `/v1/` path.
- Nullable and unknown media payloads become explicit domain variants instead of panics.
- GET and idempotent PUT requests use bounded retries. Non-idempotent skip commands are not replayed.
- `Retry-After` controls the coordinator backoff.

### Local data and diagnostics

- Settings and checkpoints use atomic replacement.
- Corrupt settings are preserved for diagnosis and replaced with defaults.
- Logs rotate at 5 MiB with five retained files and contain no credentials or raw API payloads.
- Reset removes settings, checkpoints, logs, cached data, and the native refresh credential.

### Supply chain and release

- pnpm and Cargo lockfiles are committed.
- pnpm explicitly allows build scripts only for esbuild.
- CI runs pnpm production audit and RustSec.
- Signing certificates and notarization credentials exist only as CI secrets.

## Residual risks

- A compromised operating-system account can inspect process memory and use the user's unlocked native
  credential store.
- Local Spotify metadata can differ briefly from account playback during device handoff.
- Spotify service or policy changes can make endpoints unavailable.
- Browser authorization depends on the user's default browser and Spotify account security.
- macOS signing/notarization and both platform-specific smoke tests require real release credentials
  and physical runners before a public release.
