---
title: Testing OTA Updates Locally
---

# Testing OTA Updates Locally

This guide walks you through setting up a local environment to test OTA updates end-to-end — from bundle creation to signature verification to asset serving.

---

## Prerequisites

- [minisign](https://jedisct1.github.io/minisign/) for signing bundles
- [Node.js](https://nodejs.org/) for the test server
- Tauri CLI: `cargo install tauri-cli`

```bash
# macOS
brew install minisign

# Other platforms: see https://jedisct1.github.io/minisign/
```

---

## 1. Generate a signing keypair

```bash
minisign -G -W -p minisign.pub -s minisign.key
```

The `-W` flag skips the password prompt (fine for local testing). Keep the public key — you'll need it for your Tauri config.

The output will show your public key:

```
Files signed using this key pair can be verified with the following command:

minisign -Vm <file> -P RWR+iJ9ehTe/IxJtbA0haSUz...
```

---

## 2. Create and sign a test bundle

Create a directory with your updated frontend assets:

```
bundle-v1/
├── index.html
├── style.css      # optional — multi-file bundles work
└── app.js         # optional
```

Then package and sign:

```bash
cd bundle-v1
tar czf ../bundle-v1.tar.gz .
minisign -Sm ../bundle-v1.tar.gz -s ../minisign.key
```

This produces `bundle-v1.tar.gz` and `bundle-v1.tar.gz.minisig`.

---

## 3. Run a local test server

Create a minimal Node.js server that implements the [server contract](server-contract.md):

```javascript
// server.mjs
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PORT = 3333;

const signature = (await readFile(join(__dirname, "bundle-v1.tar.gz.minisig"), "utf-8")).trim();
const bundlePath = join(__dirname, "bundle-v1.tar.gz");
const bundleSize = (await stat(bundlePath)).size;

const server = createServer(async (req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  console.log(`${req.method} ${url.pathname}${url.search}`);

  // Check endpoint: GET /api/ota/:currentSequence
  const checkMatch = url.pathname.match(/^\/api\/ota\/(\d+)$/);
  if (checkMatch) {
    const currentSeq = parseInt(checkMatch[1], 10);

    if (currentSeq >= 1) {
      res.writeHead(204);
      res.end();
      return;
    }

    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({
      version: "0.1.0-ota.1",
      sequence: 1,
      min_binary_version: "0.1.0",
      url: `http://localhost:${PORT}/bundles/bundle-v1.tar.gz`,
      signature,
      notes: "Test OTA update",
      pub_date: new Date().toISOString(),
      bundle_size: bundleSize,
    }));
    return;
  }

  // Bundle download
  const bundleMatch = url.pathname.match(/^\/bundles\/(.+)$/);
  if (bundleMatch) {
    try {
      const data = await readFile(join(__dirname, bundleMatch[1]));
      res.writeHead(200, {
        "Content-Type": "application/gzip",
        "Content-Length": data.length,
      });
      res.end(data);
    } catch {
      res.writeHead(404);
      res.end("Not found");
    }
    return;
  }

  res.writeHead(404);
  res.end("Not found");
});

server.listen(PORT, () => console.log(`Test server on http://localhost:${PORT}`));
```

Run it:

```bash
node server.mjs
```

---

## 4. Configure your app

In `tauri.conf.json`:

```json
{
  "plugins": {
    "hotswap": {
      "endpoint": "http://localhost:3333/api/ota/{{current_sequence}}",
      "pubkey": "YOUR_PUBLIC_KEY_FROM_STEP_1",
      "require_https": false
    }
  }
}
```

> `require_https: false` is required for `http://localhost`. Never disable this in production.

---

## 5. Build and run

> **Important:** `cargo tauri ios dev` and `cargo tauri android dev` proxy all asset requests to the dev server and bypass the `Assets` trait entirely. OTA asset serving is **not tested** in dev mode on mobile. Always use production builds for testing.

### Desktop (macOS, Windows, Linux)

```bash
cargo tauri build --debug
# Then run the binary from target/debug/
```

### iOS

```bash
# Requires Xcode with a signing identity
cargo tauri ios build --debug
```

Install on simulator:

```bash
xcrun simctl install booted path/to/hotswap-example.app
xcrun simctl launch booted com.example.hotswap
```

### Android

```bash
cargo tauri android build --debug
```

Install on emulator:

```bash
# Forward the test server port to the emulator
adb reverse tcp:3333 tcp:3333

# Install and launch
adb install -r path/to/app-universal-debug.apk
adb shell am start -n com.example.hotswap/.MainActivity
```

---

## 6. Verify the flow

1. **App starts** with embedded assets (the version bundled in the binary)
2. **Check for Update** — the app hits your local server and finds seq 1
3. **Apply Update** — downloads the bundle, verifies the minisign signature, extracts to disk
4. **Reload** — the app now serves the OTA assets from the filesystem
5. **Rollback** — returns to the previous version (or embedded assets)
6. **Restart the app** — OTA assets persist across restarts; if `notifyReady()` wasn't called, auto-rollback kicks in
