# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.0.5] — 2026-11-09

### Added

- **`{{channel}}` endpoint placeholder** — replaced with the active channel, for static hosting with one manifest per channel (e.g. `https://cdn.example.com/{{channel}}/latest.json`). For such URLs a 404 means "no release for this channel yet" and is treated as no update; for other URLs a 404 stays an error

### Fixed

- **Bundles re-downloaded on every launch** — the default `DiscardOnUpgrade` policy discarded any bundle whose `min_binary_version` was below the app version, so it was downloaded again on each start. Bundles now record the app version that installed them and are only discarded after an actual app upgrade. Bundles installed by earlier versions are re-downloaded once more, then record it
- **Activated bundles rolled back before their first run** — with the default `SingleLaunch` policy, a bundle activated for the next launch (`activateUpdate()` without a reload) was rolled back on that launch. The trial now starts when a page is first loaded from the bundle; only then can a missing `notifyReady()` trigger a rollback
- **Atomic metadata writes** — `hotswap-meta.json` is written via a temp file and rename, so a crash mid-write can't corrupt it
- **Android package name detection** — the hotswap directory is now derived from the running process's real package name (`/proc/self/cmdline`), so builds with an `applicationIdSuffix` (e.g. `.dev`) use the correct sandbox path ([#12](https://github.com/denniskribl/tauri-plugin-hotswap/pull/12)). Thanks @msvargas! Secondary-process suffixes (`:service`) are stripped, and the configured identifier remains the fallback when no matching sandbox directory exists (e.g. custom `android:process` names).
- **Android identifiers with hyphens** — the fallback package name now replaces `-` with `_` like Tauri's Android build does, so identifiers such as `com.example.my-app` resolve to the real `com.example.my_app` sandbox
- **Android secondary users / work profiles** — the hotswap directory now lives under `/data/user/<n>/<pkg>` for non-primary users (derived from the process uid), where `/data/data` points at another user's storage and OTA updates previously failed. Primary-user paths are unchanged
- **Guest JS build with TypeScript 6** — set `ignoreDeprecations` so the tsup DTS build no longer fails on the deprecated `baseUrl` option

### Security

- **`zip` 9** — the optional `zip` feature now uses `zip` 9, which includes parsing hardening (decoder overflow, ZIP64 size and oversized-entry fixes) and clears RUSTSEC-2026-0367/0368/0369/0429 for downstream audits. The plugin's own extractor already validated entry paths, never created symlinks or applied archive permissions, and only extracts bundles after signature verification. The `zip` feature requires Rust 1.88 (unchanged since `zip` 7.4).

### Changed

- **Minimum Rust version: 1.88** — `rust-version` now states what the dependency tree already required (tauri's dependencies and `zip` need 1.88; the previously declared 1.77.2 no longer built). Nothing changes for working setups
- **Extraction tests** — tar.gz and zip extraction are now covered for normal bundles, path traversal, absolute paths and symlink entries
- **Development lockfile refresh** — this repo's `Cargo.lock` now resolves `rustls`, `h2`, `tokio`, `zstd` and `quick-xml` (via `tauri` 2.11) to versions without known advisories; this affects CI/tests only, apps resolve their own versions
- **Dependency updates** — `serde_json`, `log`, `tokio`, `tar`, `tauri-plugin` (rust-deps group), `typescript` 6 and `@tauri-apps/api` (guest-js dev deps)

## [0.0.4] — 2026-04-09

### Added

- **Cross-platform CI** — `cargo check` and `cargo test` now run on macOS and Windows in addition to Linux
- **Rustdoc CI step** — docs are built with `-D warnings` to catch broken links and missing docs
- **`documentation` field in `Cargo.toml`** — links to docs.rs from the crates.io page
- **`#[non_exhaustive]` on public types** — `Error`, `HotswapConfig`, `HotswapManifest`, `HotswapMeta`, `HotswapCheckResult`, `HotswapVersionInfo`, `DownloadProgress`, `LifecycleEvent`, `ConfirmationDecision` are now non-exhaustive, preventing new fields/variants from being semver-breaking

### Fixed

- **Yanked dependency** — bumped `fastrand` 2.4.0 → 2.4.1
- **README version drift** — quickstart now shows `0.0.4` instead of `0.0.1`
- **Split doc comment on `DiscardOnUpgrade`** — doc block was interrupted by `#[default]` attribute

### Changed

- **Crate tarball trimmed** — added `exclude` to `Cargo.toml`, reducing package from 81 files to ~32
- **CI consolidated** — merged separate Linux and cross-platform Rust jobs into a single matrix job

## [0.0.3] — 2026-04-06

### Fixed

- **Startup crash when `plugins.hotswap` is absent from `tauri.conf.json`**: Apps using `init_with_config()` or `HotswapBuilder` (without a `plugins.hotswap` JSON section) crashed on startup. Switched plugin builder config type from `HotswapConfig` to `serde_json::Value` so Tauri accepts both `null` and JSON objects during `Builder::run()`.

### Changed

- `init()`, `init_with_config()`, and `HotswapBuilder::build()` now return `HotswapPlugin<R>` (alias for `TauriPlugin<R, serde_json::Value>`)

## [0.0.2] — 2026-04-06

### Added

- **iOS platform support** — full OTA flow tested on simulator
- **Configurable OTA policy traits** — four traits replace hardcoded behavior:
  - `BinaryCachePolicy` — controls cache retention on binary upgrades (`keep_compatible`, `discard_on_upgrade`, `never_discard`)
  - `ConfirmationPolicy` — configurable grace period for `notifyReady()` (`single_launch`, `grace_period { max_unconfirmed_launches }`)
  - `RollbackPolicy` — configurable rollback target (`latest_confirmed`, `immediate_previous_confirmed`, `embedded_only`)
  - `RetentionPolicy` — configurable version retention count (`max_retained_versions`, default 2)
- **Custom policy injection** — `HotswapBuilder` setters accept `impl Policy` for all four traits, enabling custom implementations beyond the built-in enums
- New config knobs: `binary_cache_policy`, `confirmation_policy`, `rollback_policy`, `max_retained_versions`
- `HotswapMeta` gains `unconfirmed_launch_count` field (backward compatible via serde default)
- Debug logging in `HotswapAssets::get()` for diagnosing asset resolution issues
- Local testing guide (`docs/local-testing.md`) with example test server
- App Store / Google Play compliance disclaimer in README
- Mobile-compatible example app (`lib.rs` + `main.rs` split for iOS/Android)
- README included in npm package (`tauri-plugin-hotswap-api`)
- 66 new unit tests (105 total, up from 39)

### Fixed

- **Mobile crash on startup**: Plugin builder now declares `HotswapConfig` as its config type (`Builder::<R, HotswapConfig>::new("hotswap")`). Without this, Tauri's plugin system failed to deserialize `plugins.hotswap` from the config on iOS and Android, causing a crash during app initialization.

### Changed

- Return types of `init()`, `init_with_config()`, and `HotswapBuilder::build()` changed from `TauriPlugin<R>` to `TauriPlugin<R, HotswapConfig>` (required for the mobile fix; transparent to most users since the type is passed directly to `.plugin()`)
- `check_compatibility()`, `rollback()`, `cleanup_old_versions()` now accept policy trait references instead of booleans/hardcoded logic
- `HotswapBuilder` gains `binary_cache_policy()`, `confirmation_policy()`, `rollback_policy()`, `retention_policy()`, `max_retained_versions()` setters — all accept custom `impl Policy` types

### Breaking

- **Removed `discard_on_binary_upgrade`** — the config field, builder method, and legacy mapping logic are removed entirely. Migrate as follows:
  - `discard_on_binary_upgrade: true` → `binary_cache_policy: "discard_on_upgrade"` (or omit — this is the default)
  - `discard_on_binary_upgrade: false` → `binary_cache_policy: "keep_compatible"`
  - `HotswapBuilder::discard_on_binary_upgrade(true)` → `.binary_cache_policy(BinaryCachePolicyKind::DiscardOnUpgrade)`
  - If you never set `discard_on_binary_upgrade`, no action needed — default behavior is unchanged

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

[Unreleased]: https://github.com/denniskribl/tauri-plugin-hotswap/compare/v0.0.5...HEAD
[0.0.5]: https://github.com/denniskribl/tauri-plugin-hotswap/compare/v0.0.4...v0.0.5
[0.0.4]: https://github.com/denniskribl/tauri-plugin-hotswap/compare/v0.0.3...v0.0.4
[0.0.3]: https://github.com/denniskribl/tauri-plugin-hotswap/compare/v0.0.2...v0.0.3
[0.0.2]: https://github.com/denniskribl/tauri-plugin-hotswap/compare/v0.0.1...v0.0.2
[0.0.1]: https://github.com/denniskribl/tauri-plugin-hotswap/releases/tag/v0.0.1
