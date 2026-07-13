# Release checklist

## Credentials and policy

- [ ] Spotify app is registered with `http://127.0.0.1/callback`.
- [ ] `SPOTIFY_CLIENT_ID` exists in release CI.
- [ ] Spotify quota mode and platform-policy review permit the intended distribution.
- [ ] Windows code-signing certificate and password exist in CI.
- [ ] Apple Developer ID certificate, team ID, Apple ID, and app-specific password exist in CI.
- [ ] GPL source, `LICENSE`, and `THIRD_PARTY_NOTICES.md` ship with the release.

## Automated gate

- [ ] `pnpm install --frozen-lockfile`
- [ ] `pnpm check`
- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `pnpm audit --prod --audit-level high`
- [ ] `cargo audit`
- [ ] Windows and macOS native compile jobs pass.

## Windows smoke test

- [ ] NSIS and MSI install, launch, repair, and uninstall.
- [ ] Binary and installers report a valid signature.
- [ ] Tray icon survives Explorer restart.
- [ ] Left-click toggles/raises; menu Show, Hide, Reconnect, Open Logs, About, and Kill work.
- [ ] Window remains out of taskbar and Alt+Tab and stays above ordinary windows.
- [ ] Close button, Escape, and Alt+F4 hide without ending the process.
- [ ] Saved position recovers after monitor removal and DPI changes.
- [ ] GSMTC selects Spotify rather than a browser media session.
- [ ] Spotify Web API fallback controls local and remote devices.
- [ ] Credential Manager contains the refresh token; logs and webview state do not.
- [ ] Sleep/resume excludes suspended time from both timers.

## macOS smoke test

- [ ] Universal app contains arm64 and x86_64 slices.
- [ ] DMG mounts and the app launches from `/Applications`.
- [ ] `codesign --verify --deep --strict` passes.
- [ ] `spctl --assess --type execute` passes after notarization and stapling.
- [ ] No Dock icon appears; the menu-bar status item remains available.
- [ ] Panel floats, joins all Spaces, and appears beside full-screen apps.
- [ ] Command+W and Escape hide without quitting.
- [ ] Login item enable/disable works through the native service.
- [ ] Keychain contains the refresh token; logs and webview state do not.
- [ ] Sleep/resume excludes suspended time from both timers.

## Spotify behavior

- [ ] First authorization validates state and returns through dynamic loopback port.
- [ ] Refresh survives application restart without another browser prompt.
- [ ] Track, episode, advertisement, local, null, and unknown items render safely.
- [ ] Queue invalidates on item changes and displays unavailable/stale states honestly.
- [ ] 401, 403, 429, 5xx, offline, and reconnect paths match the UI state taxonomy.
- [ ] Previous, play/pause, and next debounce and reconcile with authoritative playback.
- [ ] Account device handoff does not double-count listening time.

## Publication

- [ ] Version matches `package.json`, `Cargo.toml`, and `tauri.conf.json`.
- [ ] Release notes describe current behavior and known limitations.
- [ ] Tag uses `vMAJOR.MINOR.PATCH`.
- [ ] GitHub release contains signed NSIS, MSI, universal app, and notarized DMG artifacts.
- [ ] Downloaded artifacts receive a final clean-machine verification.
