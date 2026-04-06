---
title: Design Philosophy
---

# 🧭 Design Philosophy

**Opinionated defaults. Extensible when you need it.**

This plugin ships with strong defaults so you can go from zero to OTA updates in minutes. But every layer is swappable — you're never locked into a pattern that doesn't fit your architecture.

---

## What's opinionated

These choices are baked in because they're the right default for most apps:

| Default | Why |
|---------|-----|
| **Minisign signatures** | Every bundle is verified before extraction. No opt-out — unsigned updates are a security risk. |
| **HTTPS enforced** | Non-HTTPS endpoints are rejected. Disable explicitly if you need local development. |
| **Auto-rollback** | If `notifyReady()` isn't called after an update, the next launch rolls back. Crash loops are caught automatically. |
| **Atomic operations** | Extraction goes to a temp directory, then renames. Pointer updates use temp file + rename. No half-written state. |
| **Sequence-based ordering** | Monotonic integers, not semver comparison. Simple, unambiguous, works across any versioning scheme. |
| **Filesystem-based caching** | Updates live on disk as extracted files. No database, no custom binary format, easy to inspect and debug. |

## The server contract is opinionated (but optional)

The built-in `HttpResolver` expects a specific shape from your endpoint:

| Opinion | What it means |
|---------|---------------|
| **URL template** | `{{current_sequence}}` is replaced client-side — your server receives the current sequence in the URL path, not as a body field |
| **Query params sent automatically** | `binary_version`, `platform`, `arch`, `channel` — you don't choose what's sent, but your server can ignore what it doesn't need |
| **204 = no update** | Not a JSON response with `{ available: false }` — just an empty 204. Keeps the happy path simple. |
| **200 = JSON manifest** | A flat object with `version`, `sequence`, `url`, `signature`, `min_binary_version`. No envelope, no pagination. |
| **Sequences, not semver** | Update ordering uses monotonic integers. Semver is for display only — the plugin never parses or compares version strings. |

These opinions exist because they work for 90% of apps. A simple endpoint with a database query and a CDN-hosted bundle is all you need.

**But if they don't fit** — you're not stuck. Implement `HotswapResolver` and the entire server contract disappears. The plugin only cares about getting a `HotswapManifest` back. How you get there is up to you.

---

## What's extensible

Every extension point exists because real apps needed it:

### Bring your own update source

The `HotswapResolver` trait decouples update checking from the transport layer. The built-in `HttpResolver` calls a URL. But you can implement the trait to check anywhere — a local file, a database, a custom protocol:

```rust
use tauri_plugin_hotswap::{HotswapResolver, CheckContext, HotswapManifest};

struct MyResolver { /* ... */ }

impl HotswapResolver for MyResolver {
    fn check(&self, ctx: &CheckContext)
        -> Pin<Box<dyn Future<Output = Result<Option<HotswapManifest>>> + Send>>
    {
        // Check a local SQLite DB, a gRPC service, a message queue — anything.
    }
}
```

### Runtime configuration

Channel, endpoint, and headers are all changeable at runtime via `configure()`. No rebuild, no restart:

```typescript
// Switch a beta tester to the internal channel
await configure({
  channel: 'internal',
  headers: { 'Authorization': 'Bearer user-token' }
});

// Point at a staging server for QA
await configure({
  endpoint: 'https://staging.example.com/api/updates/{{current_sequence}}'
});
```

### Split download/activate

`applyUpdate()` does everything in one call. But if you want more control — download in the background, activate on next launch, prompt the user first — use `downloadUpdate()` + `activateUpdate()` separately.

### Custom bundle format

tar.gz works out of the box. Enable `features = ["zip"]` for zip archives. The extraction layer is internal, but the manifest format is open — you control what URL the bundle lives at and how it's hosted.

---

## What this plugin is not

- **Not a CDN.** You host the bundles. S3, Cloudflare R2, your own Nginx — anything that serves files over HTTPS.
- **Not a build system.** You build and sign the bundle. The plugin handles everything after that.
- **Not a CI pipeline.** You upload and publish the manifest. The plugin checks and downloads it.

The plugin is the last mile: check → download → verify → extract → serve. Everything before that is yours.
