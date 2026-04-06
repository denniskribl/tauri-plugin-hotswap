---
title: Server Endpoint Contract
---

# 🌐 Server Endpoint Contract

This document describes what your update server needs to implement. The default `HttpResolver` makes requests following this contract.

---

## Check Request

```
GET https://your-server.com/api/updates/42?binary_version=0.1.0&platform=macos&arch=aarch64&channel=production
```

### URL

The `endpoint` from your config, with `{{current_sequence}}` replaced by the client's current sequence number (e.g. `42`).

### Query Parameters

| Param | Always sent | Description |
|-------|-------------|-------------|
| `binary_version` | ✅ | The native binary version from `tauri.conf.json` |
| `platform` | ✅ | `macos`, `windows`, `linux`, `android`, or `ios` |
| `arch` | ✅ | `x86_64`, `aarch64`, `x86`, or `arm` |
| `channel` | Only if set | The configured update channel. Only sent if a channel has been set (via config or `configure()` at runtime). If not set, the parameter is omitted entirely. |

### Custom Headers

If `headers` is configured, all headers are sent on the check request. Common patterns:

```
Authorization: Bearer eyJhbGciOi...
X-API-Key: sk_live_...
X-Device-Id: <uuid>
```

---

## Responses

### No update available

Return **204 No Content** with an empty body.

```
HTTP/1.1 204 No Content
```

### Update available

Return **200 OK** with a JSON body:

```json
{
  "version": "0.1.0-ota.3",
  "sequence": 43,
  "min_binary_version": "0.1.0",
  "url": "https://cdn.example.com/bundles/v0.1.0-ota.3/frontend.tar.gz",
  "signature": "untrusted comment: signature from minisign secret key\nRWQ...<base64>...",
  "notes": "Fixed login button on dark mode",
  "pub_date": "2026-04-05T12:00:00.000Z",
  "mandatory": false,
  "bundle_size": 2097152
}
```

### Field Reference

| Field | Required | Type | Description |
|-------|----------|------|-------------|
| `version` | ✅ | `string` | Display version. Not used for comparison — purely for UI. |
| `sequence` | ✅ | `number` | Monotonic counter. **Higher = newer.** This is the only value used for ordering. |
| `min_binary_version` | ✅ | `string` | Minimum binary version required (semver). The client rejects the update if the running binary is older. |
| `url` | ✅ | `string` | HTTPS URL to the `.tar.gz` or `.zip` asset bundle. |
| `signature` | ✅ | `string` | Minisign signature. Either raw (starting with `untrusted comment:`) or base64-encoded. |
| `notes` | ❌ | `string` | Release notes. Passed through to the frontend for display. |
| `pub_date` | ❌ | `string` | Publication date (RFC 3339 / ISO 8601). Informational only — not used for ordering. |
| `mandatory` | ❌ | `boolean` | Hint to the frontend that this update should be applied immediately (e.g. security patch). The plugin does **not** enforce this — your frontend decides. |
| `bundle_size` | ❌ | `number` | Bundle size in bytes. Exposed so the frontend can warn users on metered connections. |

---

## Versioning Strategy

The `sequence` field is a monotonic counter — the only thing the plugin compares. Versions are **not** compared with semver.

Recommended convention:

| Event | Version | Sequence |
|-------|---------|----------|
| Binary release v0.1.0 | `0.1.0` | — |
| First hotfix | `0.1.0-ota.1` | 1 |
| Second hotfix | `0.1.0-ota.2` | 2 |
| Next binary release v0.2.0 | `0.2.0` | — |
| First hotfix for v0.2.0 | `0.2.0-ota.1` | 3 |

> 💡 Sequences are global, not per-binary-version. Your server should only return updates where `min_binary_version <= client's binary_version`.

---

## Binary Compatibility

The plugin enforces compatibility at three levels:

1. **Server-side** (your responsibility): Only return manifests where `min_binary_version <= binary_version` from the query param.
2. **Client-side (download)**: The plugin rejects the manifest if the running binary is older than `min_binary_version`.
3. **Client-side (startup)**: Cached assets are discarded if the binary was upgraded past the cache's `min_binary_version` (configurable via `discard_on_binary_upgrade`).

---

## Download Request

When the user calls `applyUpdate()` or `downloadUpdate()`, the plugin fetches the bundle from the `url` in the manifest:

```
GET https://cdn.example.com/bundles/v0.1.0-ota.3/frontend.tar.gz
Authorization: Bearer eyJhbGciOi...
```

Custom headers are sent on download requests too. The response must be the raw bundle file (`.tar.gz` or `.zip`).

---

## Example Server (Node.js)

A minimal update endpoint:

```javascript
app.get('/api/updates/:currentSequence', async (req, res) => {
  const { currentSequence } = req.params;
  const { binary_version, platform, arch, channel } = req.query;

  const latest = await db.getLatestUpdate({
    platform,
    arch,
    channel: channel || 'production',
    minBinaryVersion: { $lte: binary_version },
  });

  if (!latest || latest.sequence <= Number(currentSequence)) {
    return res.status(204).send();
  }

  res.json({
    version: latest.version,
    sequence: latest.sequence,
    min_binary_version: latest.minBinaryVersion,
    url: latest.bundleUrl,
    signature: latest.signature,
    notes: latest.notes,
    mandatory: latest.mandatory,
    bundle_size: latest.bundleSize,
  });
});
```
