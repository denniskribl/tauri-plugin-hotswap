---
title: Security
---

# 🛡️ Security

## Threat Model

The hotswap plugin serves frontend assets from the filesystem instead of the binary. This creates a new attack surface: if an attacker can tamper with the cached assets, they control the WebView.

The plugin is designed with the assumption that **the network is hostile** and **the filesystem may be tampered with between launches**.

---

## Mitigations

| Threat | Mitigation |
|--------|------------|
| **Tampered bundle on CDN** | Every bundle is verified with a **minisign signature** before extraction. The public key is compiled into your binary. |
| **MITM / downgrade attack** | HTTPS is enforced by default (`require_https: true`). Non-HTTPS URLs are rejected at both init and download time. |
| **Oversized bundle (DoS)** | `Content-Length` checked upfront. Streaming download aborted if actual bytes exceed `max_bundle_size` (default 512 MB). |
| **Malicious archive (path traversal)** | Every archive entry is validated: no `..` components, no absolute paths, must resolve within the extraction directory. Both tar.gz and zip. Leading `./` components are permitted (standard output of `tar -C dir .`). |
| **Asset key escape** | Every asset key lookup in `HotswapAssets::get()` is validated before filesystem access. Only `Component::Normal` path components are allowed. |
| **Corrupted pointer file** | The `current` pointer must match the `seq-N` format. No path separators, no traversal. Validated on every read. |
| **Crash loop after update** | The `notifyReady()` heartbeat pattern: an update is "unconfirmed" until the app calls `notifyReady()`. If the app crashes before that, the next launch automatically rolls back. |
| **Stale cache after binary upgrade** | If the binary version is older than the cached bundle's `min_binary_version`, the cache is discarded. |
| **Partial extraction (disk full, crash)** | Bundles are extracted to a `.tmp-seq-N` temp directory first, then atomically renamed. Failed extractions are cleaned up. |
| **Non-atomic pointer update** | The `current` pointer is written via temp file + `rename()` for crash safety. |
| **File permission escalation** | On Unix, `hotswap-meta.json` is written with `0o600` permissions (owner-only). |
| **Transient network failure** | Downloads retry with exponential backoff (configurable, default 3 attempts). |

---

## What the Plugin Does NOT Protect Against

- **Compromised signing key**: If your minisign private key is leaked, an attacker can sign malicious bundles. Guard your private key as you would an SSL certificate.
- **Compromised binary**: If the native binary itself is tampered with (e.g. the public key is replaced), all bets are off. This is a native code integrity problem, not an OTA problem.
- **Local filesystem access by root/admin**: A user (or malware) with root access can modify files in `{app_data}/hotswap/`. The plugin validates integrity on startup, but cannot prevent modifications between process runs.

---

## Signing Guide

### Generate a keypair

Use the Tauri CLI (recommended) or minisign directly:

```bash
# Tauri CLI (generates .key and .pub files)
pnpm tauri signer generate -w ~/.tauri/hotswap.key

# Or with minisign
minisign -G -p hotswap.pub -s hotswap.key
```

### Sign a bundle

```bash
# Tauri CLI
pnpm tauri signer sign frontend.tar.gz -k ~/.tauri/hotswap.key

# Or with minisign
minisign -Sm frontend.tar.gz -s hotswap.key
```

This produces `frontend.tar.gz.sig`. Read the contents of the `.sig` file — that's the `signature` field in your manifest.

### Configure the public key

The **public key** (`RW...` line) goes in your config:

```json
{
  "plugins": {
    "hotswap": {
      "pubkey": "<YOUR_MINISIGN_PUBKEY>"
    }
  }
}
```

> ⚠️ The **private key** should NEVER be in your repository or config files. Store it in CI secrets or a secure vault.

---

## Signature Format

The plugin accepts minisign signatures in two formats:

1. **Raw minisign format** (starts with `untrusted comment:`):
   ```
   untrusted comment: signature from minisign secret key
   RWQ...<base64 signature>...==
   trusted comment: timestamp:1234567890
   base64signatureoftheabove==
   ```

2. **Base64-encoded** (the raw format above, base64-encoded as a single string). This is what the Tauri CLI signer produces.

Both formats are auto-detected.
