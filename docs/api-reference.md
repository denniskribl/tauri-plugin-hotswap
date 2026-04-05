# 📘 API Reference

## JavaScript API

Install:

```bash
npm install tauri-plugin-hotswap-api
```

All functions are `async` and return `Promise`s.

---

### `checkUpdate()`

Check for an available update.

```typescript
import { checkUpdate } from 'tauri-plugin-hotswap-api';

const result = await checkUpdate();
if (result.available) {
  console.log(`v${result.version} available (${result.bundle_size} bytes)`);
  if (result.mandatory) {
    // Force update for security patches
  }
}
```

**Returns:** `HotswapCheckResult`

```typescript
interface HotswapCheckResult {
  available: boolean;
  version: string | null;
  sequence: number | null;
  notes: string | null;
  mandatory: boolean | null;   // true = security patch, frontend should force
  bundle_size: number | null;  // bytes, for "50MB on mobile data" warnings
}
```

---

### `applyUpdate()`

Download, verify, extract, and activate in one step. Call `checkUpdate()` first.

```typescript
await applyUpdate();
window.location.reload(); // serve new assets
```

**Returns:** `string` — the new version string.

After `applyUpdate()`, the in-memory asset provider is immediately swapped to the new version. A `window.location.reload()` will serve the new assets **without an app restart**.

> ℹ️ The update is **not auto-confirmed**. Call `notifyReady()` on the next launch to confirm. If you don't, the next launch will auto-rollback.

---

### `downloadUpdate()`

Download and verify **without activating**. The in-memory asset provider is **not** changed — the app continues serving the current version until you call `activateUpdate()`.

Use for "download now, apply later" workflows:

```typescript
// Download in background while user works
await downloadUpdate();

// Later, when the user is ready:
await activateUpdate();
window.location.reload();

// Or: don't activate at all — it will take effect on next app launch
```

**Returns:** `string` — the downloaded version string.

---

### `activateUpdate()`

Activate a previously downloaded update. The in-memory asset provider is immediately swapped — a `window.location.reload()` serves the new assets without an app restart.

```typescript
await activateUpdate();
window.location.reload();
```

**Returns:** `string` — the activated version string.

**Throws** if `downloadUpdate()` hasn't been called first.

---

### `rollback()`

Roll back to the previous version or embedded assets.

```typescript
const info = await rollback();
console.log(info.active ? `Rolled back to v${info.version}` : 'Using embedded assets');
```

**Returns:** `HotswapVersionInfo`

---

### `getVersionInfo()`

Get the current version state.

```typescript
const info = await getVersionInfo();
console.log(`Active: ${info.active}, Version: ${info.version}, Binary: ${info.binary_version}`);
```

**Returns:** `HotswapVersionInfo`

```typescript
interface HotswapVersionInfo {
  version: string | null;       // active hotswap version, null if using embedded
  sequence: number;             // active sequence, 0 if using embedded
  binary_version: string;       // native binary version from tauri.conf.json
  active: boolean;              // true if serving hotswap assets
}
```

---

### `notifyReady()`

Confirm the current version works. **Call this on every app startup.**

If an update was applied but `notifyReady()` is never called (e.g. the new assets crash the app), the next launch automatically rolls back.

```typescript
// First thing in your app initialization:
await notifyReady();
```

---

### `configure(options)`

Update runtime configuration. All fields are optional — only the fields you provide are changed. Takes effect on the next `checkUpdate()` call; no restart required.

```typescript
import { configure } from 'tauri-plugin-hotswap-api';

// Switch to the beta channel
await configure({ channel: 'beta' });

// Override the endpoint and add a header
await configure({
  endpoint: 'https://beta.example.com/api/updates/{{current_sequence}}',
  headers: { 'Authorization': 'Bearer <new-token>' },
});

// Clear the channel (no channel param sent on next check)
await configure({ channel: null });
```

**Parameters:** `ConfigureOptions`

```typescript
interface ConfigureOptions {
  channel?: string | null;   // Update channel. null clears it.
  endpoint?: string | null;  // Override the check endpoint. null resets to config value.
  headers?: Record<string, string | null>; // Merge: string values set/overwrite, null values remove that key, omitted keys unchanged.
}
```

---

### `getConfig()`

Get the current runtime configuration.

```typescript
import { getConfig } from 'tauri-plugin-hotswap-api';

const config = await getConfig();
console.log(config.channel);   // "beta" | null
console.log(config.endpoint);  // current endpoint string
```

**Returns:** `RuntimeConfig`

```typescript
interface RuntimeConfig {
  channel: string | null;
  endpoint: string | null;
  headers: Record<string, string>;
}
```

---

### `onDownloadProgress(handler)`

Listen for download progress during `applyUpdate()` or `downloadUpdate()`.

```typescript
const unlisten = await onDownloadProgress((p) => {
  const pct = p.total ? Math.round((p.downloaded / p.total) * 100) : null;
  console.log(pct ? `${pct}%` : `${p.downloaded} bytes`);
});

await applyUpdate();
unlisten();
```

```typescript
interface DownloadProgress {
  downloaded: number;       // bytes downloaded so far
  total: number | null;     // total bytes (from Content-Length), if known
}
```

---

### `onLifecycle(handler)`

Listen for lifecycle events. Use this to forward telemetry to your analytics backend.

```typescript
const unlisten = await onLifecycle((e) => {
  analytics.track('hotswap_event', e);
});
```

```typescript
interface LifecycleEvent {
  event: string;
  version?: string;
  sequence?: number;
  error?: string;
}
```

**Events emitted:**

| Event | When | Includes |
|-------|------|----------|
| `check-start` | Before resolver check | — |
| `check-complete` | After check (success) | version, sequence (if available) |
| `check-error` | Check failed | error |
| `download-start` | Before download begins | version, sequence |
| `download-complete` | After download + signature verified | version, sequence |
| `download-error` | Download failed (after all retries) | version, sequence, error |
| `apply` | Update activated | version, sequence |
| `rollback` | Rollback completed | version, sequence (of rolled-back-to) |
| `ready-confirmed` | `notifyReady()` called | version, sequence |

---

## Rust API

### Custom Resolvers

Implement `HotswapResolver` to use any update source:

```rust
use tauri_plugin_hotswap::{HotswapResolver, CheckContext, HotswapManifest};
use tauri_plugin_hotswap::error::Result;
use std::pin::Pin;
use std::future::Future;

struct MyResolver {
    // your state
}

impl HotswapResolver for MyResolver {
    fn check(
        &self,
        ctx: &CheckContext,
    ) -> Pin<Box<dyn Future<Output = Result<Option<HotswapManifest>>> + Send>> {
        let seq = ctx.current_sequence;
        let platform = ctx.platform;
        let arch = ctx.arch;
        let channel = ctx.channel.clone();

        Box::pin(async move {
            // Query your database, call your API, read a file, etc.
            // Return Ok(Some(manifest)) if an update is available,
            // Ok(None) if not.
            Ok(None)
        })
    }
}
```

Use it with the builder:

```rust
let (plugin, context) = HotswapBuilder::new("pubkey...")
    .resolver(MyResolver { /* ... */ })
    .build(context)?;
```

### Error Types

```rust
use tauri_plugin_hotswap::Error;

match err {
    Error::Network(msg) => {},           // HTTP request failed
    Error::Http { status, message } => {},// Non-2xx response
    Error::BundleTooLarge { size, limit } => {}, // Exceeded max_bundle_size
    Error::Signature(msg) => {},         // Minisign verification failed
    Error::Extraction(msg) => {},        // Archive corrupt or path traversal
    Error::InvalidManifest(msg) => {},   // JSON parse failed
    Error::Version(msg) => {},           // Semver parse failed
    Error::Config(msg) => {},            // Missing/invalid config
    Error::NoPending => {},              // No check() before apply()
    Error::InsecureUrl(url) => {},       // HTTP URL with require_https=true
    Error::Io(err) => {},               // Filesystem error
    Error::Serialization(msg) => {},     // JSON serialization failed
    Error::LockPoisoned => {},           // Internal mutex error
}
```
