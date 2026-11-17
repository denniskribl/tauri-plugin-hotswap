# 0001: Network layer and TLS for 1.0

- **Status:** Accepted
- **Date:** 2026-11-17
- **Issues:** #18 (TLS features), #26 (HTTP trait), #27 (Android trust store, after 1.0)

## Context

Up to 0.0.x, the plugin picks the TLS backend per target in `Cargo.toml`: `native-tls` on desktop and iOS, `rustls-tls` on Android. Apps cannot change it:

- Apps that must avoid OpenSSL on Linux patch the plugin to use rustls.
- Apps that must avoid `ring` (and already link OpenSSL on Android) patch it to use `native-tls` on Android.

Cargo cannot express "a different default per target that an app can still turn off" in one crate. A renamed dependency on the same package fails with "depends on crate `reqwest` multiple times with different names". An optional same-name dependency in a target table is merged with the main one, so its features leak to every target. Only a separate helper crate would work.

Two more facts shape the decision:

- **reqwest 0.13** (Dec 2025) is the maintained line. Its rustls support verifies certificates with the OS (rustls-platform-verifier). The official `tauri-plugin-updater` and `tauri-plugin-http` default to rustls on all platforms. `tauri-plugin-updater` uses the `ring` provider rather than `aws-lc-rs`.
- **`reqwest` is in our public API** through `HttpResolver::with_client` and `StaticFileResolver::with_client`. A stable 1.0 shouldn't expose a 0.x dependency (Rust API Guidelines, C-STABLE): every reqwest major release would force a hotswap major release.

## Decision

1. **TLS features that mean the same thing on every platform:**
   - `rustls-tls` (**default**): rustls with the `ring` provider. Certificates are verified by the OS on desktop and iOS; Android uses the bundled webpki roots, as 0.0.x does.
   - `native-tls`: the OS TLS stack. On Android, the app links OpenSSL.
   - `native-tls-vendored`: the same, with OpenSSL built from source.
   - No TLS feature is a `compile_error!`. If both are enabled, `native-tls` wins: enabling it on top of the defaults is an explicit choice.
2. **reqwest 0.13**, used internally only.
3. **The plugin defines its own HTTP trait,** used for both the update check and bundle downloads, with a reqwest-based default. `reqwest` types leave the public API. Apps that need a proxy, mTLS, extra root certificates or the Android system trust store provide their own implementation.
4. **No Kotlin/Android code in the plugin for 1.0.** The Android platform verifier needs a Kotlin component, Gradle setup, a ProGuard rule and JNI initialisation. If any of that is missing, it panics on the first HTTPS request, and Cargo can't detect this at build time. It can be added later in 1.x as an opt-in feature without breaking anything.

## Consequences

- **Desktop/iOS apps on default features** move from the OS TLS stack to rustls. They keep the same trust store. Servers that only offer TLS 1.0/1.1 stop working. Linux no longer needs libssl. `native-tls` restores the old behaviour.
- **Android apps on default features:** no change.
- **Apps with `default-features = false`** must choose a TLS feature.
- **Apps that call `with_client(reqwest::Client)`** implement the HTTP trait instead (see `MIGRATION.md`).
- **reqwest upgrades** become internal changes, not breaking ones.
- **Android with rustls** doesn't trust user-installed or private CAs unless the app provides its own HTTP implementation.

## Rejected alternatives

- **Keep the per-target split and add `android-native-tls`.** rustls and `ring` stay compiled in on Android, so it doesn't solve the request.
- **A helper crate for per-target defaults.** It works, but adds a second crate to publish and version forever and is non-standard.
- **`native-tls` as the default everywhere.** It breaks Android apps on default features (they'd suddenly need OpenSSL) and adds a second TLS stack next to the official plugins.
- **Keep `reqwest::Client` in the public API.** That ties our major version to reqwest's.

## Policy for future requests

- A Cargo feature only when it changes what gets compiled in. Runtime behaviour goes through the HTTP trait or configuration.
- No types from 0.x crates in the public API.
- Every supported platform and feature combination is checked in CI.
- The shape is agreed in the issue (and recorded here if it's a design decision) before a PR.
