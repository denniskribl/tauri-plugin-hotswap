# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.2] — 2026-04-06

### Added

- iOS platform support
- Debug logging in `HotswapAssets::get()` for diagnosing asset resolution issues
- Local testing guide (`docs/local-testing.md`) with test server example
- App Store / Google Play compliance disclaimer in README
- Mobile-compatible example app (`lib.rs` + `main.rs` split for iOS/Android)
- README included in npm package (`tauri-plugin-hotswap-api`)
- 34 new unit tests (73 total, up from 39): asset fallback chains, `check_update` with mock resolver, signature verification, extraction edge cases, compatibility scenarios

### Fixed

- **Mobile crash on startup**: Plugin builder now declares `HotswapConfig` as its config type (`Builder::<R, HotswapConfig>::new("hotswap")`). Without this, Tauri's plugin system failed to deserialize `plugins.hotswap` from the config on iOS and Android, causing a crash during app initialization.

### Changed

- Return types of `init()`, `init_with_config()`, and `HotswapBuilder::build()` changed from `TauriPlugin<R>` to `TauriPlugin<R, HotswapConfig>` (required for the mobile fix; transparent to most users since the type is passed directly to `.plugin()`)

## [0.0.1] — 2026-04-05

Initial release. Open-source OTA frontend updates for Tauri v2.

### Added

#### Core

- Hot-swap frontend assets at runtime — no binary rebuild, no app store review
- Minisign signature verification on every downloaded bundle
- Automatic rollback if `notifyReady()` is not called after update
- Binary compatibility gating via `min_binary_version`
- Sequence-based update ordering (monotonic integers, not semver)

#### Update Flow

- `checkUpdate()` → `applyUpdate()` one-liner for simple integrations
- Split `downloadUpdate()` + `activateUpdate()` for download-now-apply-later workflows
- Download progress events (`hotswap://download-progress`)
- Lifecycle events (`hotswap://lifecycle`) for telemetry (Sentry, PostHog, etc.)
- Download retry with exponential backoff (1s → 2s → 4s → 8s, configurable)
- `mandatory` and `bundle_size` fields in manifest for UI decisions

#### Configuration

- Configure via `tauri.conf.json`, programmatic `HotswapConfig`, or `HotswapBuilder`
- Runtime configuration via `configure()` / `getConfig()` — change channel, endpoint, and headers without restart
- Update channels (`production`, `staging`, `beta`, etc.) switchable at runtime
- Custom HTTP headers on check and download requests (auth tokens, API keys)
- Platform and architecture sent automatically on every check request

#### Extensibility

- `HotswapResolver` trait — bring your own update source
- Built-in `HttpResolver` for dynamic API endpoints
- Built-in `StaticFileResolver` for static manifest files
- Zip bundle support via `features = ["zip"]`

#### Security

- HTTPS enforced by default (configurable)
- Configurable maximum bundle size (default 512 MB)
- Path traversal protection in archive extraction (`..` and absolute paths rejected)
- Atomic extraction via temp directory + rename
- Atomic pointer updates via temp file + rename
- Restrictive file permissions on metadata (`0o600` on Unix)
- Pointer file validation (`seq-N` format enforced)
- Stale cache discarded on binary upgrade (`discard_on_binary_upgrade`)

#### Platforms

- macOS, Windows, Linux, Android

#### Guest JS (`tauri-plugin-hotswap-api`)

- `checkUpdate()`, `applyUpdate()`, `downloadUpdate()`, `activateUpdate()`
- `rollback()`, `getVersionInfo()`, `notifyReady()`
- `configure()`, `getConfig()`
- `onDownloadProgress()`, `onLifecycle()`

[Unreleased]: https://github.com/denniskribl/tauri-plugin-hotswap/compare/v0.0.2...HEAD
[0.0.2]: https://github.com/denniskribl/tauri-plugin-hotswap/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/denniskribl/tauri-plugin-hotswap/releases/tag/v0.0.1
