# 🔥🔄 tauri-plugin-hotswap

✨ Ship frontend fixes in minutes

`tauri-plugin-hotswap` is a fully open-source OTA plugin for Tauri v2 that lets you update frontend assets without rebuilding your native binary. Bring your own server, bring your own signing keys, and keep full control of your release process. 🚀

## Why teams use hotswap

- **⚡ Fast iteration**: push frontend patches without shipping a new native build
- **🔐 Security first**: signed bundles, HTTPS by default, and strict validation
- **🛟 Safe rollouts**: automatic rollback if an update is not confirmed healthy
- **🧩 No vendor lock-in**: self-host on your own infrastructure

## What this book gives you

- **[Design Philosophy](philosophy.md)** — Principles and tradeoffs behind the plugin
- **[Configuration](configuration.md)** — Initialization patterns and all runtime options
- **[API Reference](api-reference.md)** — Complete JavaScript and Rust API surface
- **[Server Contract](server-contract.md)** — Exact check endpoint and manifest requirements
- **[Creating Bundles](creating-bundles.md)** — Build, sign, and publish update bundles
- **[Architecture](architecture.md)** — Internal flow and lifecycle behavior
- **[Security](security.md)** — Threat model, mitigations, and key-management guidance

## Start here

If you are new to the project:

1. Read **Configuration**
2. Implement your endpoint via **Server Contract**
3. Follow **Creating Bundles**
4. Wire your frontend with **API Reference**

For quick install snippets and repository context, see the [GitHub README](https://github.com/denniskribl/tauri-plugin-hotswap).
