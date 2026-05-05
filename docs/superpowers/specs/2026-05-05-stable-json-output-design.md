# Stable JSON Output for Settings Files

**Date:** 2026-05-05
**Status:** Approved (pending implementation)

## Problem

Toggling a plugin in CCPM rewrites the relevant settings file (`~/.claude/settings.json`, `./.claude/settings.json`, or `./.claude/settings.local.json`) with effectively random key ordering. A field that was on row 43 may end up on row 156 after the next toggle.

Root cause: `Settings.enabled_plugins` is a `HashMap<String, bool>` and `Settings.other` is a `HashMap<String, serde_json::Value>` flattened into the parent struct (see `src/plugin/config.rs:8-14`). `HashMap` iteration order is non-deterministic, so `serde_json::to_string_pretty` produces a different layout on every write.

The same problem affects `KnownMarketplaces.marketplaces`, which is also a `HashMap`.

## Goal

Every CCPM-authored settings file looks structurally identical across writes and across projects. Toggling a plugin produces a minimal, predictable diff: the toggled boolean changes, nothing else moves.

## Approach

**Type-level fix.** Replace each user-facing `HashMap` with `BTreeMap` in the config structs. `BTreeMap` serializes in sorted key order, so output becomes deterministic at the type level — no extra logic to maintain, no risk of regression.

Nested objects inside the flattened `other` field (e.g. `permissions`, `hooks`) are represented as `serde_json::Value`. By default `serde_json::Value::Object` is `BTreeMap`-backed, so those nested maps are *already* alphabetized on serialize. Arrays inside those values (`permissions.allow`, `permissions.deny`, hook ordering, etc.) are left untouched — array order is meaningful.

A trailing newline is added to written files to match POSIX file conventions and editor expectations.

### One-time migration

The first write after this change reorders the file once; subsequent writes are byte-stable. This is the accepted trade-off for "consistency across all projects forever" — chosen over preserving each user's existing layout.

### Out of scope

- `installed_plugins.json` — CCPM only reads this file, never writes it. No change needed.
- Indentation policy (still 2-space pretty-print) and JSON-with-comments / JSONC support.
- Migration tooling — files are self-healing on the next CCPM write.

## Changes

### `src/plugin/config.rs`

Three type swaps:

```rust
use std::collections::BTreeMap;

pub struct Settings {
    #[serde(default)]
    pub enabled_plugins: BTreeMap<String, bool>,        // was HashMap

    #[serde(flatten)]
    pub other: BTreeMap<String, serde_json::Value>,     // was HashMap
}

pub struct KnownMarketplaces {
    #[serde(flatten)]
    pub marketplaces: BTreeMap<String, MarketplaceEntry>, // was HashMap
}
```

All existing callsites use `insert`, `get`, `contains_key`, `is_empty`, and iteration — every one of these has identical syntax and semantics on `BTreeMap`. No other code changes required.

### `src/plugin/operations.rs`

Append a trailing newline in `write_json_atomic`:

```rust
file.write_all(json.as_bytes())?;
file.write_all(b"\n")?;
```

## Output shape

Any `settings*.json` written by CCPM after this change:

```json
{
  "enabledPlugins": {
    "plugin-a@market": true,
    "plugin-b@market": false
  },
  "env": { ... },
  "hooks": { ... },
  "model": "...",
  "permissions": {
    "allow": [...],
    "ask": [...],
    "deny": [...]
  }
}
```

`enabledPlugins` is the named struct field and serde emits named fields before flattened ones, so it lands first. All flattened top-level fields and all nested object keys are alphabetical. Arrays preserve their existing order.

## Tests

### Unit tests in `src/plugin/config.rs`

1. **`test_settings_serialize_keys_sorted`** — Construct a `Settings` with plugins inserted in non-alphabetical order (`zebra@m`, `alpha@m`, `mid@m`). Serialize. Assert the resulting JSON string places them in alphabetical order.

2. **`test_other_fields_sorted`** — Construct a `Settings` with `other` keys inserted as `zebra`, `alpha`, `model`. Serialize. Assert top-level field order in the JSON is `enabledPlugins`, `alpha`, `model`, `zebra`.

3. **`test_settings_roundtrip_idempotent`** — Read a JSON string, deserialize, re-serialize, deserialize again, re-serialize. Assert both serialized outputs are byte-identical.

4. **`test_known_marketplaces_serialize_keys_sorted`** — Same pattern as #1 but for `KnownMarketplaces.marketplaces`.

### Unit tests in `src/plugin/operations.rs`

5. **`test_toggle_preserves_canonical_order`** — Pre-populate a settings file with `other` fields and several plugins. Call `toggle_at_scope`. Re-read the file and assert it matches the byte-for-byte canonical alphabetical layout (i.e., toggle didn't drift the ordering and didn't reintroduce `HashMap` randomness anywhere).

6. **`test_write_appends_trailing_newline`** — After any successful write, assert the file ends with `\n`.

## Risk

- **Low.** `HashMap` → `BTreeMap` is a drop-in for the operations CCPM performs; no API depends on hash semantics.
- One-time visible diff for users with existing settings files — accepted.
- No external behavior change for plugin enable/disable logic.
