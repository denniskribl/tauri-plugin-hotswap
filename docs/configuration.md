# ⚙️ Configuration

There are three ways to configure the plugin, from simplest to most flexible.

---

## Option A: `tauri.conf.json` (recommended)

```json
{
  "plugins": {
    "hotswap": {
      "endpoint": "https://example.com/api/updates/{{current_sequence}}",
      "pubkey": "<YOUR_MINISIGN_PUBKEY>",
      "channel": "production",
      "headers": {
        "Authorization": "Bearer <token>"
      },
      "max_bundle_size": 536870912,
      "max_retries": 3,
      "require_https": true,
      "discard_on_binary_upgrade": true
    }
  }
}
```

```rust
let context = tauri::generate_context!();
let (plugin, context) = tauri_plugin_hotswap::init(context)?;
```

## Option B: Programmatic config

```rust
use tauri_plugin_hotswap::HotswapConfig;

let (plugin, context) = tauri_plugin_hotswap::init_with_config(
    context,
    HotswapConfig::new("<YOUR_MINISIGN_PUBKEY>")
        .endpoint("https://example.com/api/updates/{{current_sequence}}")
        .channel("production")
        .header("Authorization", "Bearer <token>"),
)?;
```

## Option C: Builder with custom resolver

```rust
use tauri_plugin_hotswap::{HotswapBuilder, StaticFileResolver};

let (plugin, context) = HotswapBuilder::new("<YOUR_MINISIGN_PUBKEY>")
    .resolver(StaticFileResolver::new("https://cdn.example.com/latest.json"))
    .channel("production")
    .header("Authorization", "Bearer <token>")
    .max_bundle_size(256 * 1024 * 1024)
    .max_retries(5)
    .require_https(true)
    .discard_on_binary_upgrade(false)
    .build(context)?;
```

---

## 📋 Configuration Reference

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `endpoint` | `string` | **required** | Update check URL. `{{current_sequence}}` is replaced with the current sequence number at runtime. |
| `pubkey` | `string` | **required** | Minisign public key (`RW...` base64 line from your `.pub` file). |
| `channel` | `string` | — | Update channel (e.g. `"production"`, `"staging"`, `"beta"`). Sent as a `&channel=` query param. Can be changed at runtime via `configure()`. |
| `headers` | `object` | — | Custom HTTP headers sent on **every** check and download request. Common use: `{"Authorization": "Bearer <token>"}`. |
| `max_bundle_size` | `number` | `536870912` (512 MB) | Maximum download size in bytes. Downloads exceeding this are aborted immediately. Protects against memory exhaustion. |
| `max_retries` | `number` | `3` | Number of download retry attempts. Uses exponential backoff: 1s, 2s, 4s, 8s, capped at 16s. |
| `require_https` | `boolean` | `true` | Reject non-HTTPS URLs for both check and download requests. Set to `false` only for local development with `http://localhost`. |
| `discard_on_binary_upgrade` | `boolean` | `true` | When the binary version is newer than the cached bundle's `min_binary_version`, discard the cache and fall back to embedded assets. Set to `false` if you want cached bundles to persist across binary upgrades. |

---

## 🏷️ Channels

Channels let you route different users to different update streams.

### Configure at build time

```json
{
  "plugins": {
    "hotswap": {
      "channel": "production"
    }
  }
}
```

### Switch at runtime

```typescript
import { configure, getConfig } from 'tauri-plugin-hotswap-api';

// Opt into beta updates
await configure({ channel: 'beta' });

// Check what channel we're on
const config = await getConfig();
console.log(config.channel); // "beta"

// Reset to default (no channel param sent)
await configure({ channel: null });
```

The channel is sent as a `&channel=beta` query parameter on check requests. Your server decides what to return for each channel.

---

## 🔗 Runtime Endpoint Override

You can switch the update endpoint at runtime without restarting the app:

```typescript
import { configure } from 'tauri-plugin-hotswap-api';

// Point to a different update server at runtime
await configure({
  endpoint: 'https://staging.example.com/api/updates/{{current_sequence}}',
});

// Reset to the endpoint from tauri.conf.json
await configure({ endpoint: null });
```

The override takes effect on the next `checkUpdate()` call.

---

## 🔑 Custom Headers

Headers are sent on both check and download requests. Use them for:

- **Auth tokens**: `{"Authorization": "Bearer <jwt>"}`
- **API keys**: `{"X-API-Key": "sk_..."}`
- **Device identification**: `{"X-Device-Id": "..."}`

### From `tauri.conf.json`

```json
{
  "plugins": {
    "hotswap": {
      "headers": {
        "Authorization": "Bearer eyJhbGciOi..."
      }
    }
  }
}
```

### From Rust

```rust
HotswapConfig::new("pubkey...")
    .endpoint("https://...")
    .header("Authorization", "Bearer eyJhbGciOi...")
    .header("X-API-Key", "sk_live_...")
```

### Update headers at runtime

```typescript
import { configure } from 'tauri-plugin-hotswap-api';

// Merge headers: set or overwrite a key (other existing headers are kept)
await configure({
  headers: { 'Authorization': 'Bearer <refreshed-token>' },
});

// Remove a specific header by passing null for its value
await configure({
  headers: { 'Authorization': null },
});
```

`configure({ headers })` uses **merge semantics**: keys with a string value are added or overwritten, keys with a `null` value are removed, and any headers not mentioned in the call are left unchanged.

> ⚠️ Headers are stored in memory only — they are not persisted to disk. Sensitive tokens should be loaded from secure storage at startup.
