---
title: Advanced Policies
---

# Advanced Policies

The plugin exposes four configurable policy traits that govern OTA lifecycle decisions. Most users only need the config knobs in `tauri.conf.json` — this guide is for advanced customization via the `HotswapBuilder`.

---

## Policy Overview

| Policy | What it controls | Default |
|--------|-----------------|---------|
| `BinaryCachePolicy` | Whether to discard cached OTA on binary version change | `discard_on_upgrade` |
| `ConfirmationPolicy` | Grace period before rollback if `notifyReady()` not called | `single_launch` |
| `RollbackPolicy` | Which version to roll back to | `latest_confirmed` |
| `RetentionPolicy` | How many versions to keep on disk | 2 |

---

## Config-based (simple path)

```json
{
  "plugins": {
    "hotswap": {
      "binary_cache_policy": "keep_compatible",
      "confirmation_policy": { "grace_period": { "max_unconfirmed_launches": 3 } },
      "rollback_policy": "latest_confirmed",
      "max_retained_versions": 3
    }
  }
}
```

---

## Builder-based (advanced path)

```rust
use tauri_plugin_hotswap::{
    HotswapBuilder, BinaryCachePolicyKind, ConfirmationPolicyKind,
    RollbackPolicyKind, StaticFileResolver,
};

let (plugin, context) = HotswapBuilder::new("<PUBKEY>")
    .resolver(StaticFileResolver::new("https://cdn.example.com/latest.json"))
    .binary_cache_policy(BinaryCachePolicyKind::KeepCompatible)
    .confirmation_policy(ConfirmationPolicyKind::GracePeriod {
        max_unconfirmed_launches: 3,
    })
    .rollback_policy(RollbackPolicyKind::LatestConfirmed)
    .max_retained_versions(3)
    .build(context)?;
```

---

## Binary Cache Policy

Controls whether a cached OTA bundle is discarded when the binary version changes.

| Variant | Behavior |
|---------|----------|
| `keep_compatible` | Keep if `binary >= min_binary_version`. Recommended for most apps. |
| `discard_on_upgrade` | Discard when `binary > min_binary_version`. Legacy default. |
| `never_discard` | Never discard based on binary version. |

The safety check (`binary < min_binary_version` → always discard) is enforced outside the policy and cannot be overridden.

---

## Confirmation Policy

Controls what happens at startup when the current OTA version hasn't been confirmed via `notifyReady()`.

| Variant | Behavior |
|---------|----------|
| `single_launch` | Rollback immediately on first unconfirmed launch (default). |
| `grace_period { max_unconfirmed_launches: N }` | Allow N unconfirmed launches before rollback. |

Threshold: rollback when `unconfirmed_launch_count >= max_unconfirmed_launches`. Setting `max_unconfirmed_launches: 0` behaves the same as `single_launch`.

---

## Rollback Policy

Controls which version the plugin rolls back to.

| Variant | Behavior |
|---------|----------|
| `latest_confirmed` | Highest-sequence confirmed version. Default. |
| `immediate_previous_confirmed` | The confirmed version just below current. |
| `embedded_only` | Always fall back to embedded assets. |

---

## Retention Policy

Controls how many OTA versions are retained on disk.

`max_retained_versions` is the **total** count, including current and rollback candidate. Minimum is 2 (clamped automatically). Current and rollback candidate are always preserved as a safety floor.

| `max_retained_versions` | Versions on disk |
|--------------------------|------------------|
| 2 (default) | current + rollback candidate |
| 3 | current + rollback + 1 older |
| 5 | current + rollback + 3 older |

---

## Safety Invariants

These behaviors are **not** configurable via policy traits:

- Minisign signature verification is always mandatory
- Archive path validation (`..` and absolute paths rejected)
- Atomic extraction (temp dir + rename)
- Atomic pointer updates (temp file + rename)
- Binary too old check (`binary < min_binary_version` → always discard)
