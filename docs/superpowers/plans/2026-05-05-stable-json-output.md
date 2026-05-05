# Stable JSON Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every CCPM-authored settings file deterministic and alphabetically sorted, so a plugin toggle produces a minimal diff and the file shape is identical across projects.

**Architecture:** Type-level fix only. Replace `HashMap` with `BTreeMap` in three places in `src/plugin/config.rs` so serde serializes keys in sorted order. Append a trailing newline in `write_json_atomic`. No behavior changes outside the on-disk byte layout.

**Tech Stack:** Rust (stable, MSRV 1.70), `serde`, `serde_json`, `std::collections::BTreeMap`. Tests run with `cargo test`; lint with `cargo clippy -- -D warnings`; format with `cargo fmt --check`.

**Spec:** `docs/superpowers/specs/2026-05-05-stable-json-output-design.md`

**Project quality gates (CI-enforced):**
- `cargo test` — all tests pass
- `cargo clippy -- -D warnings` — zero warnings
- `cargo fmt --check` — formatted

**Commit style:** Free-form descriptive messages, no Conventional Commits prefixes.

## Subagent guardrails (binding for every task)

Any subagent — implementer, reviewer, or otherwise — dispatched for this plan MUST obey these rules. They override anything else in this plan or your usual habits:

1. **Never run destructive git commands without first asking the user.** This includes (non-exhaustive): `git stash`, `git checkout <ref> -- <file>`, `git checkout -- <file>`, `git restore`, `git reset`, `git clean`, `git rebase`, `git revert`, `git push --force`, `git branch -D`, `git worktree remove`. If a task seems to require any of these, STOP and report back as `BLOCKED` with the exact command and the reason you think it's needed. The user reviews and either authorizes or supplies an alternative.
2. **Read-only git is fine** — `git status`, `git log`, `git diff`, `git show`, `git rev-parse`, `git ls-files`, `git blame`, `git branch --show-current`. These never alter project state.
3. **`git add` and `git commit` are allowed** because the plan asks for them, but only with the file paths the task explicitly enumerates. Do not `git add -A` or `git add .`.
4. **Never silence errors with `2>/dev/null`.** If a command might fail, you need to see the failure, not hide it.
5. **Stay in scope.** A task names exact files. Do not modify or even read-via-checkout any file outside that list. If a verification step makes you think you need a file outside scope, that's a signal to stop and report it — not to widen the change.

These rules exist because of a real failure: a reviewer subagent on a previous task proposed `git stash; git checkout <old-sha> -- tests/integration.rs; git checkout <branch> -- tests/integration.rs; git stash pop 2>/dev/null` while reviewing a change that did not touch `tests/integration.rs` at all. The user caught it. They will not always catch it. Therefore: ask first.

---

## File Structure

| File | Responsibility | Change |
|------|----------------|--------|
| `src/plugin/config.rs` | Defines `Settings`, `KnownMarketplaces` and their serde behavior | Modify: 3 `HashMap` → `BTreeMap` field swaps; add ordering tests |
| `src/plugin/operations.rs` | Atomic file writes, plugin toggle logic | Modify: append `\n` in `write_json_atomic`; add trailing-newline + canonical-order tests |
| `CLAUDE.md` | High-level project context | Modify: short note in the Architecture section about deterministic on-disk JSON layout |
| `CLAUDE.local.md` | Low-level implementation notes (gitignored) | Modify: add a "Stable JSON Output" subsection |
| `FEATURE_PLAN.md` | Feature tracking | Modify: add a Completed Features entry |
| `docs/architecture.md` | Detailed architecture | Modify: one-paragraph note under the config section |

No new files. Tests are co-located in their source modules per project convention.

---

## Task 1: Set up feature branch and baseline

**Files:** none (git only)

- [ ] **Step 1: Confirm clean working tree on `main`**

Run: `git status`
Expected:
```
On branch main
Your branch is up to date with 'origin/main'.
nothing to commit, working tree clean
```

If not clean, stop and surface the diff to the user before continuing.

- [ ] **Step 2: Create and switch to feature branch**

Run: `git checkout -b feature/stable-json-output`
Expected: `Switched to a new branch 'feature/stable-json-output'`

- [ ] **Step 3: Run baseline tests so we know what "green before any change" looks like**

Run: `cargo test --lib`
Expected: all tests pass (no failures, no compile errors). Note the count for later comparison.

- [ ] **Step 4: Run baseline clippy**

Run: `cargo clippy -- -D warnings`
Expected: clean exit (no warnings).

---

## Task 2: Add deterministic key ordering for `Settings.enabled_plugins`

**Files:**
- Modify: `src/plugin/config.rs:1-14` (imports + `Settings` struct)
- Test: `src/plugin/config.rs` (co-located `#[cfg(test)] mod tests`)

- [ ] **Step 1: Add the failing test**

Open `src/plugin/config.rs`, find the existing `mod tests` block (around line 186), and append this test at the end of the module (just before the closing `}`):

```rust
    #[test]
    fn test_settings_serialize_keys_sorted() {
        // Insert in non-alphabetical order so HashMap (random) would almost
        // certainly NOT produce alphabetical output by accident.
        let mut settings = Settings::default();
        for id in &[
            "zebra@m",
            "alpha@m",
            "mango@m",
            "kiwi@m",
            "papaya@m",
            "banana@m",
        ] {
            settings.enabled_plugins.insert((*id).to_string(), true);
        }

        let json = serde_json::to_string_pretty(&settings).unwrap();

        // Find the line indices of each plugin id in the serialized output.
        let positions: Vec<(&str, usize)> = [
            "alpha@m", "banana@m", "kiwi@m", "mango@m", "papaya@m", "zebra@m",
        ]
        .iter()
        .map(|id| (*id, json.find(id).unwrap_or_else(|| panic!("missing {id}"))))
        .collect();

        // Each id must appear at a strictly increasing byte offset, i.e. alphabetical order.
        for window in positions.windows(2) {
            assert!(
                window[0].1 < window[1].1,
                "expected {} before {} in serialized JSON, got positions {} and {}",
                window[0].0,
                window[1].0,
                window[0].1,
                window[1].1,
            );
        }
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib test_settings_serialize_keys_sorted`
Expected: test runs but FAILS most of the time (HashMap ordering is non-deterministic; with 6 keys the chance of accidental alphabetical order is negligible). Acceptable failure messages: an `assert!` panic about "expected X before Y".

If this run accidentally passes (~1 in 720 chance), re-run several times — at least one run must fail to confirm the test exercises the right behavior. If it never fails, the test is broken; revisit Step 1.

- [ ] **Step 3: Apply the type swap**

In `src/plugin/config.rs`, replace the imports and `Settings` struct:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Claude Code settings.json structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub enabled_plugins: BTreeMap<String, bool>,

    #[serde(flatten)]
    pub other: BTreeMap<String, serde_json::Value>,
}
```

Note: only `enabled_plugins`'s type changed from `HashMap` to `BTreeMap` for THIS task. We are also pre-emptively swapping `other` to `BTreeMap` in the same edit because both fields will need it (a half-finished struct mixing the two types compiles but leaves Task 3's test failing — bundling the swap into one diff is the cleanest path). The `HashMap` import can stay if other code still uses it; if the file no longer references `HashMap`, remove that import.

- [ ] **Step 4: Search the file for any leftover `HashMap` references and adjust**

Run: `grep -n "HashMap" src/plugin/config.rs`
- If lines 1-2 still show `use std::collections::HashMap;` and no other code in the file uses `HashMap`, delete that line.
- If other code still uses `HashMap` (e.g., `InstalledPlugins.plugins`), keep the import — `BTreeMap` is added alongside it.

- [ ] **Step 5: Verify the test passes**

Run: `cargo test --lib test_settings_serialize_keys_sorted`
Expected: PASS, deterministically (run 3 times to confirm).

- [ ] **Step 6: Run the full test suite**

Run: `cargo test`
Expected: all tests pass. If any test fails because it relied on `HashMap` iteration order (unlikely — all existing usages use `insert`/`get`/`contains_key`/iteration of all entries), fix the test by sorting the iteration result before assertion, then re-run.

- [ ] **Step 7: Commit**

```bash
git add src/plugin/config.rs
git commit -m "Sort enabledPlugins and other top-level keys deterministically

Switch Settings.enabled_plugins and Settings.other from HashMap to
BTreeMap so serde emits keys in alphabetical order on every write.
This stops settings files from reshuffling on every plugin toggle."
```

---

## Task 3: Add deterministic key ordering for `KnownMarketplaces.marketplaces`

**Files:**
- Modify: `src/plugin/config.rs` (the `KnownMarketplaces` struct, ~line 50)
- Test: `src/plugin/config.rs` (co-located tests)

- [ ] **Step 1: Add the failing test**

Append to `mod tests` in `src/plugin/config.rs`:

```rust
    #[test]
    fn test_known_marketplaces_serialize_keys_sorted() {
        let mut marketplaces = KnownMarketplaces::default();
        let entries = [
            "zebra-mp",
            "alpha-mp",
            "mango-mp",
            "kiwi-mp",
            "papaya-mp",
            "banana-mp",
        ];
        for name in &entries {
            marketplaces.marketplaces.insert(
                (*name).to_string(),
                MarketplaceEntry {
                    source: MarketplaceSource {
                        source: "github".into(),
                        repo: "owner/repo".into(),
                    },
                    install_location: PathBuf::from("/tmp"),
                    last_updated: "2026-05-05T00:00:00Z".into(),
                    auto_update: false,
                },
            );
        }

        let json = serde_json::to_string_pretty(&marketplaces).unwrap();

        let positions: Vec<(&str, usize)> = [
            "alpha-mp", "banana-mp", "kiwi-mp", "mango-mp", "papaya-mp", "zebra-mp",
        ]
        .iter()
        .map(|n| (*n, json.find(n).unwrap_or_else(|| panic!("missing {n}"))))
        .collect();

        for window in positions.windows(2) {
            assert!(
                window[0].1 < window[1].1,
                "expected {} before {} in serialized JSON",
                window[0].0,
                window[1].0,
            );
        }
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib test_known_marketplaces_serialize_keys_sorted`
Expected: FAIL with the assert message (HashMap order is random).

- [ ] **Step 3: Apply the type swap**

In `src/plugin/config.rs`, change the `KnownMarketplaces` struct (around line 49):

```rust
/// Known marketplaces tracking file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnownMarketplaces {
    #[serde(flatten)]
    pub marketplaces: BTreeMap<String, MarketplaceEntry>,
}
```

(Only the `HashMap` → `BTreeMap` substitution.)

- [ ] **Step 4: Verify the test passes**

Run: `cargo test --lib test_known_marketplaces_serialize_keys_sorted`
Expected: PASS, deterministically.

- [ ] **Step 5: Verify the existing `test_known_marketplaces_deserialize` still passes**

Run: `cargo test --lib test_known_marketplaces_deserialize`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/plugin/config.rs
git commit -m "Sort known_marketplaces keys deterministically

Switch KnownMarketplaces.marketplaces from HashMap to BTreeMap so
auto-update toggles no longer reshuffle the file."
```

---

## Task 4: Add roundtrip idempotency test

**Files:**
- Test only: `src/plugin/config.rs`

This test pins the invariant that "deserialize → serialize → deserialize → serialize" produces the same bytes. With BTreeMap in place, this should already pass — we are adding a regression guard.

- [ ] **Step 1: Add the test**

Append to `mod tests` in `src/plugin/config.rs`:

```rust
    #[test]
    fn test_settings_roundtrip_idempotent() {
        let original_json = r#"{
  "enabledPlugins": {
    "zebra@m": true,
    "alpha@m": false,
    "mango@m": true
  },
  "model": "claude-sonnet-4-6",
  "env": {
    "FOO": "bar",
    "ALPHA": "1"
  },
  "permissions": {
    "allow": ["one", "two"],
    "deny": ["three"]
  }
}"#;

        let parsed: Settings = serde_json::from_str(original_json).unwrap();
        let first = serde_json::to_string_pretty(&parsed).unwrap();

        let reparsed: Settings = serde_json::from_str(&first).unwrap();
        let second = serde_json::to_string_pretty(&reparsed).unwrap();

        assert_eq!(
            first, second,
            "second serialization differed from first — non-deterministic ordering somewhere"
        );
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test --lib test_settings_roundtrip_idempotent`
Expected: PASS (BTreeMap and `serde_json::Value`'s default `BTreeMap` backing combine to give a stable layout).

If this fails: investigate which field is reordering. The default features of `serde_json` should keep `Value::Object` sorted. Verify `Cargo.toml` does NOT enable the `preserve_order` feature on `serde_json` — if it does, that opt-in would defeat the determinism we need. Disable it.

- [ ] **Step 3: Commit**

```bash
git add src/plugin/config.rs
git commit -m "Add roundtrip idempotency test for Settings serialization

Pins the invariant that re-serializing a parsed Settings produces
byte-identical output, guarding against future regressions that
reintroduce non-deterministic map types."
```

---

## Task 5: Append trailing newline in `write_json_atomic`

**Files:**
- Modify: `src/plugin/operations.rs:255-265` (`write_json_atomic`)
- Test: `src/plugin/operations.rs` (co-located tests)

- [ ] **Step 1: Add the failing test**

Append to `mod tests` in `src/plugin/operations.rs`:

```rust
    #[test]
    fn test_write_appends_trailing_newline() {
        let (_temp, service) = setup_test_env();

        service
            .enable_plugin("trailing-newline@market", Scope::User)
            .unwrap();

        let bytes = fs::read(service.paths.user_settings()).unwrap();
        assert!(
            bytes.ends_with(b"\n"),
            "settings file should end with a newline; last 4 bytes were {:?}",
            &bytes[bytes.len().saturating_sub(4)..],
        );
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --lib test_write_appends_trailing_newline`
Expected: FAIL — current implementation does not append `\n`.

- [ ] **Step 3: Modify `write_json_atomic` to append a newline**

In `src/plugin/operations.rs`, find this block (around line 255):

```rust
        file.write_all(json.as_bytes())
            .map_err(|source| PluginError::ConfigWriteError {
                path: temp_path.clone(),
                source,
            })?;
```

Replace with:

```rust
        file.write_all(json.as_bytes())
            .map_err(|source| PluginError::ConfigWriteError {
                path: temp_path.clone(),
                source,
            })?;

        file.write_all(b"\n")
            .map_err(|source| PluginError::ConfigWriteError {
                path: temp_path.clone(),
                source,
            })?;
```

- [ ] **Step 4: Verify the test passes**

Run: `cargo test --lib test_write_appends_trailing_newline`
Expected: PASS.

- [ ] **Step 5: Run the full test suite**

Run: `cargo test`
Expected: all tests pass — including the integration tests in `tests/integration.rs`. If any integration test was asserting exact file byte-for-byte content without the trailing newline, update it.

- [ ] **Step 6: Commit**

```bash
git add src/plugin/operations.rs
git commit -m "Append trailing newline to written settings files

Match POSIX file conventions and editor expectations. Also reduces
diff noise when the file is later edited by hand."
```

---

## Task 6: Add toggle-preserves-canonical-order integration test

**Files:**
- Test only: `src/plugin/operations.rs`

End-to-end check: write a populated settings file, toggle one plugin, and assert the resulting file is byte-identical to a freshly-serialized canonical version. This proves no `HashMap` randomness slipped back in via any code path.

- [ ] **Step 1: Add the test**

Append to `mod tests` in `src/plugin/operations.rs`:

```rust
    #[test]
    fn test_toggle_preserves_canonical_order() {
        let (_temp, service) = setup_test_env();

        // Pre-populate with several plugins and several "other" fields,
        // inserted in non-alphabetical order.
        let mut settings = Settings::default();
        for id in &["zebra@m", "alpha@m", "mango@m"] {
            settings.enabled_plugins.insert((*id).to_string(), true);
        }
        settings.other.insert(
            "model".to_string(),
            serde_json::Value::String("claude-sonnet-4-6".into()),
        );
        settings.other.insert(
            "env".to_string(),
            serde_json::json!({ "FOO": "bar" }),
        );

        let path = service.paths.user_settings();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, serde_json::to_string_pretty(&settings).unwrap()).unwrap();

        // Toggle one of the plugins.
        service.disable_plugin("alpha@m", Scope::User).unwrap();

        // Build the expected canonical layout: same fields, alpha@m disabled.
        let mut expected = settings.clone();
        expected.enabled_plugins.insert("alpha@m".to_string(), false);
        let mut expected_bytes = serde_json::to_string_pretty(&expected)
            .unwrap()
            .into_bytes();
        expected_bytes.push(b'\n');

        let actual_bytes = fs::read(&path).unwrap();

        assert_eq!(
            actual_bytes, expected_bytes,
            "toggle produced a non-canonical byte layout — ordering or formatting drifted"
        );
    }
```

- [ ] **Step 2: Run the test**

Run: `cargo test --lib test_toggle_preserves_canonical_order`
Expected: PASS — Tasks 2 and 5 already supplied the needed behavior.

- [ ] **Step 3: Commit**

```bash
git add src/plugin/operations.rs
git commit -m "Add end-to-end test that toggling preserves canonical layout

Writes a populated settings file, toggles a plugin, and asserts the
resulting bytes match a freshly-serialized canonical version. Catches
any regression that reintroduces non-deterministic map ordering or
drops the trailing newline."
```

---

## Task 7: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Run the full unit test suite**

Run: `cargo test --lib`
Expected: every test passes; total count >= baseline from Task 1 + 4 (the new tests added across Tasks 2–6).

- [ ] **Step 2: Run integration tests**

Run: `cargo test --test integration`
Expected: every test passes.

- [ ] **Step 3: Run clippy with zero-warning enforcement**

Run: `cargo clippy -- -D warnings`
Expected: clean exit, no warnings.

(Note: `cargo clippy --all-targets -- -D warnings` would surface 10 pre-existing deprecation errors in `tests/integration.rs` — `assert_cmd::Command::cargo_bin` is deprecated in favor of `cargo::cargo_bin_cmd!`. This is unrelated to stable-JSON-output and is out of scope for this plan. The CCPM project's documented quality gate in `CLAUDE.md` is `cargo clippy -- -D warnings` without `--all-targets`, which is what this step enforces.)

- [ ] **Step 4: Verify formatting**

Run: `cargo fmt --check`
Expected: no output, exit code 0.

If any of Steps 1–4 fails, fix the issue and re-run all four before continuing.

- [ ] **Step 5: Manual smoke check on a real settings file**

Run from a project root that has plugins:
```bash
cargo run --release -- list -s local
cargo run --release -- enable <some-plugin-id>
cat .claude/settings.local.json | head -30
cargo run --release -- disable <some-plugin-id>
diff <(cat .claude/settings.local.json) <(cat .claude/settings.local.json)
```

Expected: the cat output shows alphabetically-sorted keys with `enabledPlugins` near the top, and toggling produces a one-line diff (the boolean flip). File ends with a newline.

If you don't have a project with plugins handy, skip this step and rely on the automated tests.

---

## Task 8: Update documentation

**Files:**
- Modify: `CLAUDE.md` — Architecture section
- Modify: `CLAUDE.local.md` — add Stable JSON Output subsection
- Modify: `FEATURE_PLAN.md` — Completed Features
- Modify: `docs/architecture.md` — config files note

- [ ] **Step 1: Update `CLAUDE.md`**

Find the section beginning `### Config Files Read` and immediately after the closing of that table, append:

```markdown

### On-Disk JSON Format

CCPM-authored settings files use a deterministic alphabetical key order so a plugin toggle produces a minimal diff and files look identical across projects:
- `enabledPlugins` is the first top-level key.
- All other top-level keys (flattened from `Settings.other`) follow in alphabetical order.
- Nested object keys are alphabetical at every level.
- Arrays (e.g. `permissions.allow`, `permissions.deny`, `permissions.ask`, hook arrays) preserve their existing order.
- Files end with a trailing newline.

This is enforced at the type level in `src/plugin/config.rs` via `BTreeMap`. CCPM does not modify `installed_plugins.json`, so its layout is untouched.
```

- [ ] **Step 2: Update `CLAUDE.local.md`**

Find the heading `## Low-Level Implementation Details`. Just before the next `---` separator, append:

```markdown

### Stable JSON Output (COMPLETED 2026-05-05)

**Files modified:**
- `src/plugin/config.rs` — `Settings.enabled_plugins`, `Settings.other`, and `KnownMarketplaces.marketplaces` switched from `HashMap` to `BTreeMap`.
- `src/plugin/operations.rs` — `write_json_atomic` appends a trailing newline.

**Why:** `HashMap` iteration order is non-deterministic, so each plugin toggle re-emitted the file with a randomly-permuted layout. Editors and reviewers lost their mental model. Switching to `BTreeMap` gives sorted-key serialization at the type level — no extra logic to maintain.

**Why BTreeMap and not `IndexMap` + insertion order:** the user explicitly asked for the same shape across projects, not preservation of each project's existing order. Alphabetical wins because it's the only ordering that is identical across machines.

**Why nested objects also work:** `serde_json::Value::Object` is `BTreeMap`-backed by default. As long as `Cargo.toml` does NOT enable `serde_json`'s `preserve_order` feature, nested objects inside `other` (e.g. `permissions`, `hooks`) are sorted automatically.

**Tests added (co-located):**
- `test_settings_serialize_keys_sorted`
- `test_known_marketplaces_serialize_keys_sorted`
- `test_settings_roundtrip_idempotent`
- `test_write_appends_trailing_newline`
- `test_toggle_preserves_canonical_order`

**One-time migration:** the first write after upgrading reorders an existing settings file once; subsequent writes are byte-stable.
```

- [ ] **Step 3: Update `FEATURE_PLAN.md`**

Find the `## Completed Features` heading (line 252) and append a new entry under it (above any horizontal rule that follows):

```markdown

### Stable JSON Output (COMPLETED 2026-05-05)

**Status**: Complete

CCPM-authored settings files now serialize with a deterministic alphabetical key order at every level, plus a trailing newline. Plugin toggles produce minimal diffs; files look identical across projects.

**Files**: `src/plugin/config.rs`, `src/plugin/operations.rs`

**Spec**: `docs/superpowers/specs/2026-05-05-stable-json-output-design.md`
**Plan**: `docs/superpowers/plans/2026-05-05-stable-json-output.md`
```

- [ ] **Step 4: Update `docs/architecture.md`**

Open `docs/architecture.md` and find the section that documents settings file layout (search for "settings.json" or "Settings"). Add a one-paragraph note in the relevant subsection:

```markdown

**On-disk format:** Settings files are written with deterministic alphabetical key ordering (via `BTreeMap`) and a trailing newline. This guarantees minimal diffs across plugin toggles and identical layout across projects. Nested object keys are also alphabetized; arrays preserve their order.
```

If `docs/architecture.md` does not have a settings-file-layout subsection, add one under the most relevant top-level heading (likely "Configuration" or "Storage" or similar).

- [ ] **Step 5: Verify the full test suite still passes after doc edits**

Run: `cargo test`
Expected: PASS (docs don't affect tests, but this is a sanity check that nothing else regressed during the session).

- [ ] **Step 6: Commit**

```bash
git add CLAUDE.md CLAUDE.local.md FEATURE_PLAN.md docs/architecture.md
git commit -m "Document stable JSON output feature

Adds notes in CLAUDE.md, CLAUDE.local.md, FEATURE_PLAN.md, and
docs/architecture.md describing the deterministic-key-order guarantee
and the trailing-newline write behavior."
```

(`CLAUDE.local.md` is gitignored — `git add` will silently skip it. That's expected. The file still gets updated locally.)

---

## Task 9: Push and (optionally) open a PR

**Files:** none (git only)

- [ ] **Step 1: Push the feature branch**

Run: `git push -u origin feature/stable-json-output`
Expected: branch is created on the remote and tracking is set up.

If the user has not authorized pushing, stop here and ask before proceeding.

- [ ] **Step 2: (Optional) Open a PR**

If the user wants a PR for review, run:

```bash
gh pr create --title "Deterministic alphabetical key order in settings files" --body "$(cat <<'EOF'
## Summary
- `Settings.enabled_plugins`, `Settings.other`, `KnownMarketplaces.marketplaces` switched from `HashMap` to `BTreeMap` so settings files serialize with a stable alphabetical key order
- `write_json_atomic` now appends a trailing newline
- One-time reorder on first write; byte-stable thereafter

## Test plan
- [ ] `cargo test`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt --check`
- [ ] Toggle a plugin in a real project, confirm the diff is one line

Spec: `docs/superpowers/specs/2026-05-05-stable-json-output-design.md`
Plan: `docs/superpowers/plans/2026-05-05-stable-json-output.md`
EOF
)"
```

If the user prefers to keep the work as a local branch only, skip this step.

---

## Self-Review Notes (already applied)

- **Spec coverage:** Every spec section maps to a task. `enabled_plugins` ordering → Task 2. `other` ordering → bundled into Task 2 (note in Step 3 explains why). `marketplaces` ordering → Task 3. Roundtrip idempotency → Task 4. Trailing newline → Task 5. End-to-end toggle test → Task 6. CI gates (test/clippy/fmt) → Task 7. Documentation updates → Task 8.
- **Placeholder scan:** none.
- **Type consistency:** all method calls (`insert`, `get`, `contains_key`, `is_empty`, iteration) work identically on `HashMap` and `BTreeMap`. No callsite outside `config.rs` needs changes.
- **Quality gates:** Task 7 explicitly runs `cargo test`, `cargo clippy -- -D warnings`, and `cargo fmt --check` — the project's stated bar.
