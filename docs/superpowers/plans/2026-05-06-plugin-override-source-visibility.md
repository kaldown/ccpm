# Plugin Override-Source Visibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make CCPM's TUI details pane and `ccpm info` CLI output show the source file path (e.g. `~/Projects/tern/Ternv3/.claude/settings.local.json`) for each per-scope override flag, so a user-installed plugin with a per-project override is visible at a glance — no filesystem grep required.

**Architecture:** Pure additive change. One new helper method on `Plugin` (`project_settings_source`) plus a display variant (`project_settings_source_display`). The TUI details pane (`src/ui/details.rs`) and CLI `info` subcommand (`src/cli/mod.rs`) both call the display variant when building each Project/Local Settings row and append a ` · <path>` suffix when the source is `Some`. The User row is never annotated. No persisted state changes; no effect on enable/disable logic.

**Tech Stack:** Rust (stable, MSRV 1.70), `ratatui` (TUI), `serde`, `serde_json`, `assert_cmd` + `predicates` + `tempfile` (integration tests). Tests run with `cargo test`; lint with `cargo clippy -- -D warnings`; format with `cargo fmt --check`.

**Spec:** `docs/superpowers/specs/2026-05-06-plugin-override-source-visibility-design.md`

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

These rules exist because of a real failure on a previous CCPM task: a reviewer subagent proposed `git stash; git checkout <old-sha> -- tests/integration.rs; git checkout <branch> -- tests/integration.rs; git stash pop 2>/dev/null` while reviewing a change that did not touch `tests/integration.rs` at all. The user caught it. They will not always catch it. Therefore: ask first.

---

## File Structure

| File | Responsibility | Change |
|------|----------------|--------|
| `src/plugin/mod.rs` | `Plugin` struct + helpers | Modify: add `project_settings_source`, add `project_settings_source_display`, add unit tests |
| `src/ui/details.rs` | TUI details pane render | Modify: extract pure `build_details_lines` for testability; append ` · <path>` to Project/Local rows; add unit tests |
| `src/cli/mod.rs` | CLI subcommand handlers | Modify: extend `show_info` with a `Settings:` block (User/Project/Local) and ` · <path>` suffix on Project/Local rows |
| `tests/integration.rs` | End-to-end CLI tests | Modify: add a test asserting `ccpm info` output includes the override source path |
| `CLAUDE.md` | High-level project context | Modify: short note about override-source visibility |
| `CLAUDE.local.md` | Low-level implementation notes (gitignored) | Modify: add a "Plugin Override-Source Visibility" subsection |
| `FEATURE_PLAN.md` | Feature tracking | Modify: add a Completed Features entry |
| `docs/architecture.md` | Detailed architecture | Modify: paragraph in the UI/details rendering section |

No new files. Tests are co-located in their source modules per project convention; the integration test goes in the existing `tests/integration.rs`.

---

## Task 1: Baseline check

**Files:**
- None modified

**Goal:** Confirm we're on the right branch with a clean tree, the spec is committed, and capture the baseline test count so subsequent tasks can verify no regressions.

- [ ] **Step 1: Verify branch and clean tree**

Run:
```bash
git branch --show-current
git status
```
Expected: branch is `feature/plugin-override-source-visibility`; status shows clean tree (the spec file `docs/superpowers/specs/2026-05-06-plugin-override-source-visibility-design.md` is already committed).

- [ ] **Step 2: Confirm spec is committed**

Run:
```bash
git log --oneline -5
```
Expected: top commit is `Add design spec for plugin override-source visibility`.

- [ ] **Step 3: Run baseline test suite and record counts**

Run:
```bash
cargo test
```
Expected: all tests pass. Record the unit-test count and integration-test count from the output. Subsequent tasks will add tests; the totals should grow monotonically.

- [ ] **Step 4: Run baseline lint**

Run:
```bash
cargo clippy -- -D warnings
```
Expected: passes with zero warnings. The CI-equivalent invocation (`cargo clippy -- -D warnings`, no `--all-targets`) is what we baseline against; `cargo clippy --all-targets` surfaces 10 pre-existing deprecation errors in `tests/integration.rs` (`assert_cmd::Command::cargo_bin`) that are documented tech debt out of scope for this plan.

---

## Task 2: Add `Plugin::project_settings_source` helper

**Files:**
- Modify: `src/plugin/mod.rs` (add helper method + 6 unit tests)

**Goal:** Pure helper that, given a scope and a CWD, returns the path to the settings file that contains this plugin's flag at that scope. Returns `None` for User scope (file is always `~/.claude/settings.json`, not annotated by callers) or when no value exists at the requested scope.

- [ ] **Step 1: Add `Path` import**

In `src/plugin/mod.rs`, change the import on line 10 from:
```rust
use std::path::PathBuf;
```
to:
```rust
use std::path::{Path, PathBuf};
```

- [ ] **Step 2: Write 6 failing unit tests**

Append to the `mod tests` block in `src/plugin/mod.rs` (after the existing `test_project_path_display` test, before the closing `}` of the module):

```rust
#[test]
fn test_project_settings_source_local_install_with_project_path() {
    let mut plugin = make_test_plugin();
    plugin.install_scope = Scope::Local;
    plugin.project_path = Some(PathBuf::from("/proj"));
    plugin.enabled_local = Some(true);

    let cwd = PathBuf::from("/cwd");
    assert_eq!(
        plugin.project_settings_source(Scope::Local, &cwd),
        Some(PathBuf::from("/proj/.claude/settings.local.json"))
    );
}

#[test]
fn test_project_settings_source_user_install_with_cwd_override() {
    let mut plugin = make_test_plugin();
    plugin.install_scope = Scope::User;
    plugin.project_path = None;
    plugin.enabled_local = Some(false);

    let cwd = PathBuf::from("/cwd");
    assert_eq!(
        plugin.project_settings_source(Scope::Local, &cwd),
        Some(PathBuf::from("/cwd/.claude/settings.local.json"))
    );
}

#[test]
fn test_project_settings_source_user_scope_returns_none() {
    let mut plugin = make_test_plugin();
    plugin.enabled_user = Some(true);
    let cwd = PathBuf::from("/cwd");
    assert_eq!(plugin.project_settings_source(Scope::User, &cwd), None);
}

#[test]
fn test_project_settings_source_no_value_returns_none() {
    let plugin = make_test_plugin(); // all enabled_* are None
    let cwd = PathBuf::from("/cwd");
    assert_eq!(plugin.project_settings_source(Scope::Project, &cwd), None);
    assert_eq!(plugin.project_settings_source(Scope::Local, &cwd), None);
}

#[test]
fn test_project_settings_source_project_scope_uses_settings_json() {
    let mut plugin = make_test_plugin();
    plugin.install_scope = Scope::Project;
    plugin.project_path = Some(PathBuf::from("/proj"));
    plugin.enabled_project = Some(true);

    let cwd = PathBuf::from("/cwd");
    assert_eq!(
        plugin.project_settings_source(Scope::Project, &cwd),
        Some(PathBuf::from("/proj/.claude/settings.json"))
    );
}

#[test]
fn test_project_settings_source_local_install_without_project_path_falls_back_to_cwd() {
    let mut plugin = make_test_plugin();
    plugin.install_scope = Scope::Local;
    plugin.project_path = None; // legacy data pre-projectPath fix
    plugin.enabled_local = Some(true);

    let cwd = PathBuf::from("/cwd");
    assert_eq!(
        plugin.project_settings_source(Scope::Local, &cwd),
        Some(PathBuf::from("/cwd/.claude/settings.local.json"))
    );
}
```

- [ ] **Step 3: Run tests and verify they fail**

Run:
```bash
cargo test --lib project_settings_source
```
Expected: all 6 new tests fail with a compile error like `no method named project_settings_source found for struct Plugin`.

- [ ] **Step 4: Implement `project_settings_source`**

In `src/plugin/mod.rs`, inside the existing `impl Plugin { ... }` block (after `project_path_display` at line ~212, before the closing `}`), add:

```rust
/// Path to the settings file that supplies this plugin's flag at the given scope.
///
/// Returns `None` if no flag exists at the requested scope, or for `Scope::User`
/// (the user-scope flag always lives in `~/.claude/settings.json` — not annotated
/// by callers because that path carries no useful diagnostic information).
///
/// For project/local-scope installs the project directory is `self.project_path`.
/// For user-scope installs with a CWD override (project_path is None) the project
/// directory falls back to `cwd`.
pub fn project_settings_source(&self, scope: Scope, cwd: &Path) -> Option<PathBuf> {
    let has_value = match scope {
        Scope::User => return None,
        Scope::Project => self.enabled_project.is_some(),
        Scope::Local => self.enabled_local.is_some(),
    };
    if !has_value {
        return None;
    }

    let project_dir = self.project_path.as_deref().unwrap_or(cwd);
    let file_name = match scope {
        Scope::Project => "settings.json",
        Scope::Local => "settings.local.json",
        Scope::User => unreachable!(),
    };
    Some(project_dir.join(".claude").join(file_name))
}
```

- [ ] **Step 5: Run tests and verify they pass**

Run:
```bash
cargo test --lib project_settings_source
```
Expected: all 6 new tests pass.

- [ ] **Step 6: Run full test suite and lint**

Run:
```bash
cargo test && cargo clippy -- -D warnings
```
Expected: every test passes; clippy reports zero new warnings.

- [ ] **Step 7: Commit**

Run:
```bash
git add src/plugin/mod.rs
git commit -m "Add Plugin::project_settings_source helper"
```

---

## Task 3: Add `Plugin::project_settings_source_display` helper

**Files:**
- Modify: `src/plugin/mod.rs` (add display variant + 3 unit tests)

**Goal:** Display-formatted variant returning a `String` with home-relative `~/...` substitution when the path lives under `$HOME`. Used directly by both the TUI and CLI render paths so they share the same formatting rule.

- [ ] **Step 1: Write 3 failing unit tests**

Append to the `mod tests` block in `src/plugin/mod.rs` (after the tests added in Task 2):

```rust
#[test]
fn test_project_settings_source_display_returns_some_path_string() {
    let mut plugin = make_test_plugin();
    plugin.install_scope = Scope::Local;
    plugin.project_path = Some(PathBuf::from("/some/absolute/proj"));
    plugin.enabled_local = Some(true);

    let cwd = PathBuf::from("/cwd");
    assert_eq!(
        plugin.project_settings_source_display(Scope::Local, &cwd),
        Some("/some/absolute/proj/.claude/settings.local.json".to_string())
    );
}

#[test]
fn test_project_settings_source_display_uses_home_relative_format() {
    if let Some(home) = dirs::home_dir() {
        let mut plugin = make_test_plugin();
        plugin.install_scope = Scope::Local;
        plugin.project_path = Some(home.join("Projects/myapp"));
        plugin.enabled_local = Some(true);

        let cwd = PathBuf::from("/cwd");
        assert_eq!(
            plugin.project_settings_source_display(Scope::Local, &cwd),
            Some("~/Projects/myapp/.claude/settings.local.json".to_string())
        );
    }
}

#[test]
fn test_project_settings_source_display_returns_none_when_source_none() {
    let plugin = make_test_plugin(); // no values set
    let cwd = PathBuf::from("/cwd");
    assert_eq!(
        plugin.project_settings_source_display(Scope::Local, &cwd),
        None
    );
    assert_eq!(
        plugin.project_settings_source_display(Scope::User, &cwd),
        None
    );
}
```

- [ ] **Step 2: Run tests and verify they fail**

Run:
```bash
cargo test --lib project_settings_source_display
```
Expected: 3 new tests fail with `no method named project_settings_source_display`.

- [ ] **Step 3: Implement `project_settings_source_display`**

In `src/plugin/mod.rs`, inside the same `impl Plugin { ... }` block, immediately after `project_settings_source`, add:

```rust
/// Display-formatted source path with home-relative substitution (`~/...`).
/// Returns `None` whenever `project_settings_source` would return `None`.
pub fn project_settings_source_display(
    &self,
    scope: Scope,
    cwd: &Path,
) -> Option<String> {
    self.project_settings_source(scope, cwd).map(|p| {
        if let Some(home) = dirs::home_dir() {
            if let Ok(rel) = p.strip_prefix(&home) {
                return format!("~/{}", rel.display());
            }
        }
        p.display().to_string()
    })
}
```

- [ ] **Step 4: Run tests and verify they pass**

Run:
```bash
cargo test --lib project_settings_source_display
```
Expected: all 3 new tests pass.

- [ ] **Step 5: Run full test suite and lint**

Run:
```bash
cargo test && cargo clippy -- -D warnings
```
Expected: every test passes; clippy reports zero new warnings.

- [ ] **Step 6: Commit**

Run:
```bash
git add src/plugin/mod.rs
git commit -m "Add Plugin::project_settings_source_display with home-relative format"
```

---

## Task 4: TUI details pane — extract pure builder + render path suffix

**Files:**
- Modify: `src/ui/details.rs` (refactor + new code + 3 unit tests)

**Goal:** Extract the line-building logic from `render_details` into a pure function `build_details_lines(plugin, cwd) -> Vec<Line<'static>>` so it's unit-testable. Then append ` · <path>` to the Project and Local rows of the Settings block when `project_settings_source_display` returns `Some`.

- [ ] **Step 1: Add imports needed by the refactor**

In `src/ui/details.rs`, replace the existing imports (lines 1-8) with:

```rust
use crate::app::App;
use crate::plugin::{Plugin, Scope};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::path::{Path, PathBuf};
```

- [ ] **Step 2: Refactor only — extract `build_details_lines` without behavior change**

Replace the entire body of `render_details` (current lines 18-186) with:

```rust
pub fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let content = match app.selected_plugin() {
        Some(plugin) => build_details_lines(plugin, &cwd),
        None => vec![Line::from(Span::styled(
            "No plugin selected",
            Style::default().fg(Color::DarkGray),
        ))],
    };

    let details = Paragraph::new(content)
        .block(
            Block::default()
                .title(" Details ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: true });

    frame.render_widget(details, area);
}

pub fn build_details_lines(plugin: &Plugin, _cwd: &Path) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(plugin.name.clone()),
        ]),
        Line::from(vec![
            Span::styled(
                "Marketplace: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(plugin.marketplace.clone()),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            if plugin.is_enabled() {
                Span::styled("Enabled", Style::default().fg(Color::Green))
            } else {
                Span::styled("Disabled", Style::default().fg(Color::Red))
            },
        ]),
    ];

    let install_location = match (plugin.install_scope, plugin.is_current_project) {
        (Scope::User, _) => "User (~/.claude)".to_string(),
        (Scope::Project, true) => "Project (this project)".to_string(),
        (Scope::Project, false) => "Project (other project)".to_string(),
        (Scope::Local, true) => "Local (this project)".to_string(),
        (Scope::Local, false) => "Local (other project)".to_string(),
    };
    lines.push(Line::from(vec![
        Span::styled("Installed: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(install_location),
    ]));

    lines.push(Line::from(vec![
        Span::styled(
            "Enabled in: ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(plugin.enabled_context()),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Settings:",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::raw("  User:    "),
        format_setting(plugin.enabled_user),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Project: "),
        format_setting(plugin.enabled_project),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  Local:   "),
        format_setting(plugin.enabled_local),
    ]));

    let effective_label = match plugin.effective_scope() {
        Some(scope) => format!(
            "Effective: {} ({})",
            if plugin.is_enabled() {
                "ENABLED"
            } else {
                "DISABLED"
            },
            scope
        ),
        None => "Effective: DISABLED (no settings)".to_string(),
    };
    lines.push(Line::from(Span::styled(
        effective_label,
        Style::default()
            .fg(if plugin.is_enabled() {
                Color::Green
            } else {
                Color::Red
            })
            .add_modifier(Modifier::BOLD),
    )));

    if plugin.install_scope != Scope::User {
        if let Some(path_display) = plugin.project_path_display() {
            lines.push(Line::from(vec![
                Span::styled("Project: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(
                    path_display,
                    if plugin.is_current_project {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Yellow)
                    },
                ),
            ]));
        }
    }

    if let Some(ref version) = plugin.version {
        lines.push(Line::from(vec![
            Span::styled("Version: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(version.clone()),
        ]));
    }

    if let Some(ref author) = plugin.author {
        let author_text = if let Some(ref email) = author.email {
            format!("{} <{}>", author.name, email)
        } else {
            author.name.clone()
        };
        lines.push(Line::from(vec![
            Span::styled("Author: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(author_text),
        ]));
    }

    if let Some(ref path) = plugin.install_path {
        lines.push(Line::from(vec![
            Span::styled("Path: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                path.display().to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    if let Some(ref date) = plugin.last_updated {
        lines.push(Line::from(vec![
            Span::styled("Updated: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(date.clone()),
        ]));
    }

    lines.push(Line::from(""));
    if let Some(ref description) = plugin.description {
        lines.push(Line::from(vec![Span::styled(
            "Description:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(Span::raw(description.clone())));
    }

    lines
}
```

(Note: this version takes `_cwd: &Path` but does not use it yet — the `_` prefix silences `unused_variables`. The next steps wire it in. The `.clone()`s on plugin fields exist because the function returns `Vec<Line<'static>>`; the original took `&str` references because everything lived in the same scope. Clones are cheap; clarity over cycle-counting here.)

- [ ] **Step 3: Run full build and existing tests — verify no regression**

Run:
```bash
cargo build && cargo test
```
Expected: build succeeds; all existing tests still pass. (At this point the refactor is purely mechanical — same on-screen output, new structure.)

- [ ] **Step 4: Write 3 failing unit tests for the path suffix**

Append to `src/ui/details.rs` (at end of file):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::{Author, Plugin, Scope};

    fn line_text(line: &Line) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn make_plugin() -> Plugin {
        Plugin {
            id: "test@m".into(),
            name: "test".into(),
            marketplace: "m".into(),
            description: None,
            version: None,
            author: None::<Author>,
            install_scope: Scope::User,
            install_path: None,
            project_path: None,
            is_current_project: true,
            enabled_user: None,
            enabled_project: None,
            enabled_local: None,
            installed_at: None,
            last_updated: None,
        }
    }

    fn settings_row<'a>(lines: &'a [Line<'static>], label: &str) -> &'a Line<'static> {
        lines
            .iter()
            .find(|l| line_text(l).trim_start().starts_with(label))
            .unwrap_or_else(|| panic!("settings row not found: {label}"))
    }

    #[test]
    fn test_details_renders_path_on_local_row_when_set() {
        let mut p = make_plugin();
        p.install_scope = Scope::Local;
        p.project_path = Some(PathBuf::from("/proj"));
        p.enabled_local = Some(true);

        let lines = build_details_lines(&p, &PathBuf::from("/cwd"));
        let row = settings_row(&lines, "Local:");
        let text = line_text(row);

        assert!(text.contains(" · "), "Local row missing source separator: {text}");
        assert!(
            text.contains("settings.local.json"),
            "Local row missing source file path: {text}"
        );
    }

    #[test]
    fn test_details_skips_path_on_no_setting_rows() {
        let mut p = make_plugin();
        p.install_scope = Scope::Local;
        p.project_path = Some(PathBuf::from("/proj"));
        // enabled_project intentionally None — row should render "(no setting)" with no path

        let lines = build_details_lines(&p, &PathBuf::from("/cwd"));
        let row = settings_row(&lines, "Project:");
        let text = line_text(row);

        assert!(
            !text.contains(" · "),
            "Project row should have no source on (no setting): {text}"
        );
    }

    #[test]
    fn test_details_skips_path_on_user_row() {
        let mut p = make_plugin();
        p.enabled_user = Some(true);

        let lines = build_details_lines(&p, &PathBuf::from("/cwd"));
        let row = settings_row(&lines, "User:");
        let text = line_text(row);

        assert!(
            !text.contains(" · "),
            "User row is never annotated: {text}"
        );
    }
}
```

- [ ] **Step 5: Run new tests and verify they fail**

Run:
```bash
cargo test --lib details::tests
```
Expected: `test_details_renders_path_on_local_row_when_set` **fails** because the Local row currently never includes ` · `. The other two tests (`test_details_skips_path_on_no_setting_rows` and `test_details_skips_path_on_user_row`) **pass** by accident — they assert the absence of ` · `, which is trivially true under the current code. That's fine; they exist as regression guards once the suffix logic lands and will distinguish "we forgot the User row" from "we got the suffix in the right place."

If `test_details_renders_path_on_local_row_when_set` passes already, the refactor inadvertently introduced the suffix — STOP, diff `src/ui/details.rs`, and remove any `project_settings_source_display` calls. The next step is the only place they should appear.

- [ ] **Step 6: Add the suffix logic to Project and Local rows**

In `src/ui/details.rs` inside `build_details_lines`, locate the three Settings rows (User/Project/Local) added in Step 2. Replace the Project and Local row pushes (`lines.push(Line::from(vec![ Span::raw("  Project: "), ... ]));` and the analogous `Local:` push) with the path-aware variants:

```rust
    // Project row — annotated with source file path when set
    {
        let mut spans = vec![
            Span::raw("  Project: "),
            format_setting(plugin.enabled_project),
        ];
        if let Some(path) = plugin.project_settings_source_display(Scope::Project, _cwd) {
            spans.push(Span::styled(
                format!("  · {}", path),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(spans));
    }

    // Local row — annotated with source file path when set
    {
        let mut spans = vec![
            Span::raw("  Local:   "),
            format_setting(plugin.enabled_local),
        ];
        if let Some(path) = plugin.project_settings_source_display(Scope::Local, _cwd) {
            spans.push(Span::styled(
                format!("  · {}", path),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(spans));
    }
```

Then rename the parameter from `_cwd: &Path` to `cwd: &Path` in the `build_details_lines` signature, and update the two `_cwd` references above to `cwd`. The leading underscore was only there to silence `unused_variables` while the parameter wasn't yet wired in.

The User row (`Span::raw("  User:    "), format_setting(plugin.enabled_user)`) stays unchanged — never annotated.

- [ ] **Step 7: Run tests and verify they pass**

Run:
```bash
cargo test --lib details::tests
```
Expected: all 3 details tests pass. `test_details_renders_path_on_local_row_when_set` now finds the ` · ` separator and the `settings.local.json` substring.

- [ ] **Step 8: Run full test suite and lint**

Run:
```bash
cargo test && cargo clippy -- -D warnings
```
Expected: every test passes; clippy reports zero new warnings.

- [ ] **Step 9: Commit**

Run:
```bash
git add src/ui/details.rs
git commit -m "Show override source file path on Project/Local Settings rows in TUI"
```

---

## Task 5: CLI `info` — add Settings block + path suffix + integration test

**Files:**
- Modify: `src/cli/mod.rs` (`show_info` function)
- Modify: `tests/integration.rs` (add 1 integration test)

**Goal:** Bring `ccpm info <id>` in line with the TUI details pane: render a `Settings:` block with User/Project/Local rows, and append ` · <path>` to Project/Local rows when `project_settings_source_display` returns `Some`. The integration test asserts the override source path appears for a user-scope plugin pinned by a CWD `settings.local.json`.

- [ ] **Step 1: Write the failing integration test**

Append to `tests/integration.rs` (at end of file):

```rust
#[test]
fn test_cli_info_shows_override_source_path() {
    let home = TempDir::new().unwrap();
    let project = TempDir::new().unwrap();

    // Fake user home: gitlab installed user-scope, enabled globally
    let claude_dir = home.path().join(".claude");
    let plugins_dir = claude_dir.join("plugins");
    fs::create_dir_all(&plugins_dir).unwrap();

    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"gitlab@market":true}}"#,
    )
    .unwrap();

    fs::write(
        plugins_dir.join("installed_plugins.json"),
        r#"{
            "version": 2,
            "plugins": {
                "gitlab@market": [{
                    "scope": "user",
                    "installPath": "/fake/path",
                    "version": "1.0.0",
                    "installedAt": "2026-01-01T00:00:00Z",
                    "lastUpdated": "2026-01-01T00:00:00Z"
                }]
            }
        }"#,
    )
    .unwrap();

    // Project local override: disabled
    let project_claude = project.path().join(".claude");
    fs::create_dir_all(&project_claude).unwrap();
    fs::write(
        project_claude.join("settings.local.json"),
        r#"{"enabledPlugins":{"gitlab@market":false}}"#,
    )
    .unwrap();

    let output = Command::cargo_bin("ccpm")
        .unwrap()
        .args(["info", "gitlab@market"])
        .env("HOME", home.path())
        .current_dir(project.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "ccpm info failed: {stderr}");

    // Settings block must include a Local row showing the disabled value
    assert!(
        stdout.contains("Local:") && stdout.contains("disabled"),
        "info should show 'Local: disabled' row; got:\n{stdout}"
    );
    // The Local row should be annotated with the source file path
    assert!(
        stdout.contains(" · ") && stdout.contains("settings.local.json"),
        "info should annotate Local row with ' · <path>/settings.local.json'; got:\n{stdout}"
    );
}
```

- [ ] **Step 2: Run integration test and verify it fails**

Run:
```bash
cargo test --test integration test_cli_info_shows_override_source_path
```
Expected: test fails — current `show_info` output has no `Settings:` block and no ` · ` suffix.

- [ ] **Step 3: Modify `show_info` in `src/cli/mod.rs`**

In `src/cli/mod.rs`, locate the `show_info` function (starts at line ~202). Replace its body so it builds a Settings block and uses the display helper. The full replacement of the `Some(p) => { ... }` arm is below — leave the `match plugin` outer match and `None` arm intact:

```rust
        Some(p) => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

            println!("Name:        {}", p.name);
            println!("Marketplace: {}", p.marketplace);
            println!("ID:          {}", p.id);
            println!(
                "Status:      {}",
                if p.is_enabled() {
                    "enabled"
                } else {
                    "disabled"
                }
            );

            let installed = match (p.install_scope, p.is_current_project) {
                (Scope::User, _) => "User (~/.claude)".to_string(),
                (Scope::Project, true) => "Project (this project)".to_string(),
                (Scope::Project, false) => "Project (other project)".to_string(),
                (Scope::Local, true) => "Local (this project)".to_string(),
                (Scope::Local, false) => "Local (other project)".to_string(),
            };
            println!("Installed:   {}", installed);
            println!("Enabled in:  {}", p.enabled_context());

            // Per-scope settings breakdown — mirrors the TUI details pane.
            // Project/Local rows are annotated with the source file path when set.
            println!("Settings:");
            println!("  User:    {}", format_setting_text(p.enabled_user));
            println!(
                "{}",
                format_settings_row(
                    "Project",
                    p.enabled_project,
                    p.project_settings_source_display(Scope::Project, &cwd),
                )
            );
            println!(
                "{}",
                format_settings_row(
                    "Local",
                    p.enabled_local,
                    p.project_settings_source_display(Scope::Local, &cwd),
                )
            );

            let effective_label = match p.effective_scope() {
                Some(scope) => format!(
                    "Effective:   {} ({})",
                    if p.is_enabled() { "ENABLED" } else { "DISABLED" },
                    scope
                ),
                None => "Effective:   DISABLED (no settings)".to_string(),
            };
            println!("{}", effective_label);

            if p.install_scope != Scope::User {
                if let Some(path_display) = p.project_path_display() {
                    println!("Project:     {}", path_display);
                }
            }

            if let Some(ref version) = p.version {
                println!("Version:     {}", version);
            }

            if let Some(ref author) = p.author {
                let author_str = if let Some(ref email) = author.email {
                    format!("{} <{}>", author.name, email)
                } else {
                    author.name.clone()
                };
                println!("Author:      {}", author_str);
            }

            if let Some(ref path) = p.install_path {
                println!("Path:        {}", path.display());
            }

            if let Some(ref desc) = p.description {
                println!("\nDescription:\n{}", desc);
            }
        }
```

Then add two private helpers at the bottom of `src/cli/mod.rs` (outside any function, at module top level):

```rust
fn format_setting_text(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "enabled",
        Some(false) => "disabled",
        None => "(no setting)",
    }
}

fn format_settings_row(label: &str, value: Option<bool>, source: Option<String>) -> String {
    let value_text = format_setting_text(value);
    // Pad "Project" / "Local" to a constant column so the value text aligns.
    // Longest label is "Project" (7), so pad to 7 then a colon and 1 space.
    let label_padded = format!("{:<7}", label);
    match source {
        Some(s) => format!("  {}: {}  · {}", label_padded, value_text, s),
        None => format!("  {}: {}", label_padded, value_text),
    }
}
```

- [ ] **Step 4: Run integration test and verify it passes**

Run:
```bash
cargo test --test integration test_cli_info_shows_override_source_path
```
Expected: pass.

- [ ] **Step 5: Run full test suite and lint**

Run:
```bash
cargo test && cargo clippy -- -D warnings
```
Expected: every test passes; clippy reports zero new warnings.

- [ ] **Step 6: Spot-check the human-facing output**

Run from the CCPM project root:
```bash
cargo run --quiet -- info superpowers@claude-plugins-official
```
Expected: output now includes a `Settings:` block with `User:`, `Project:`, `Local:` rows and an `Effective:` line. If there's no override on this plugin, no `· ` annotations will appear — that's correct.

- [ ] **Step 7: Commit**

Run:
```bash
git add src/cli/mod.rs tests/integration.rs
git commit -m "Show override source file path in ccpm info Settings block"
```

---

## Task 6: Documentation updates

**Files:**
- Modify: `CLAUDE.md`
- Modify: `CLAUDE.local.md`
- Modify: `FEATURE_PLAN.md`
- Modify: `docs/architecture.md`

**Goal:** Reflect the new feature in the four documentation files per the project's documentation hierarchy in `CLAUDE.md`.

- [ ] **Step 1: Update `CLAUDE.md`**

In `CLAUDE.md`, find the `**UI Features**:` bullet list under the `### Key Concepts` section. Append a new bullet to that list:

```markdown
- Override marker `↓` (yellow) next to scope indicator when a non-install-scope setting is active
- `[overrides: N]` count in header when overrides exist in the current view
- Project path shown for all project/local scope plugins
- Details pane shows per-scope Settings block (User/Project/Local breakdown + Effective line)
- **Override source file path** shown on Project/Local rows of the Settings block (e.g. `Local: enabled  · ~/Projects/tern/Ternv3/.claude/settings.local.json`) — same suffix appears in `ccpm info` output
```

(The first four bullets already exist; the fifth is new. Verify by re-reading the file before editing.)

- [ ] **Step 2: Update `CLAUDE.local.md`**

In `CLAUDE.local.md`, find the `## Low-Level Implementation Details` section. After the existing "Stable JSON Output" subsection, append:

```markdown
### Plugin Override-Source Visibility (COMPLETED 2026-05-06)

**Files modified:**
- `src/plugin/mod.rs` — `Plugin::project_settings_source(scope, cwd)` and `Plugin::project_settings_source_display(scope, cwd)`.
- `src/ui/details.rs` — `render_details` now delegates to `build_details_lines(plugin, cwd)`, a pure function returning `Vec<Line<'static>>`. The Project and Local rows of the Settings block append a ` · <path>` suffix in DarkGray when the source is `Some`.
- `src/cli/mod.rs` — `show_info` gained a `Settings:` block (User/Project/Local) plus an `Effective:` line, mirroring the TUI. Project/Local rows carry the same ` · <path>` suffix in plain text.
- `tests/integration.rs` — `test_cli_info_shows_override_source_path` confirms the suffix appears for a user-scope plugin overridden by a CWD `settings.local.json`.

**Why:** the existing TUI/CLI rendered `Settings: Local: enabled` with no indication of *which* project's `settings.local.json` carried the flag. A plugin installed at user scope but pinned by another project's local settings was indistinguishable from a plugin pinned by the current project's local settings — leading to incidents like chrome-devtools-mcp pinned in Ternv3 while debugging from `claude-config`.

**Why home-relative format:** path `strip_prefix($HOME)` then `~/...` substitution in `project_settings_source_display`. Matches the format already used by `Plugin::project_path_display()` and the CWD label in the TUI header.

**Why a separate `_display` variant:** the raw `PathBuf` form is what tests assert on (deterministic, no env dependency); the `String` form is what renderers print (env-aware home substitution). Keeping them separate avoids embedding `dirs::home_dir()` in the type-level helper.

**Out of scope (deferred):**
- Detection of orphan caches, orphan marketplaces, half-uninstalled plugins (T2 in brainstorming).
- Shelling out to `claude plugin uninstall` for cleanup (T3).
- Inline per-row override tags in the plugin list (option C in brainstorming, recorded in project memory as the fallback if this proves insufficient).
- `ccpm list` default-output column changes — `list` is a table; `info` is the carrier.
```

- [ ] **Step 3: Update `FEATURE_PLAN.md`**

In `FEATURE_PLAN.md`, find the "Completed Features" section. Add a new entry following the existing format (check current entries for style; the date is `2026-05-06`):

```markdown
### Plugin Override-Source Visibility (2026-05-06)

The TUI details pane and `ccpm info` CLI output now show the source file path for each per-scope override flag. A user-installed plugin pinned by another project's `settings.local.json` is identifiable at a glance — the Local row of the Settings block carries `· <path-to-settings-file>`.

**Implementation:** new `Plugin::project_settings_source` + `project_settings_source_display` helpers; `render_details` extracted to a pure `build_details_lines(plugin, cwd)` builder; `show_info` gained a Settings/Effective block to match the TUI.

**Spec:** `docs/superpowers/specs/2026-05-06-plugin-override-source-visibility-design.md`

**Plan:** `docs/superpowers/plans/2026-05-06-plugin-override-source-visibility.md`
```

(If `FEATURE_PLAN.md` doesn't have a "Completed Features" section in this exact form, follow the file's existing convention — e.g., a checkbox list, a table, etc. Read the file first; do not invent a new structure.)

- [ ] **Step 4: Update `docs/architecture.md`**

In `docs/architecture.md`, find the section that describes the TUI details pane (search for "details" or "Details pane"). Add a paragraph at the end of that section:

```markdown
**Override source visibility.** The Settings block in the details pane displays the source file path for any Project or Local override on the selected plugin. The path is derived at render time by `Plugin::project_settings_source_display(scope, cwd)` — for project/local-scope installs from `plugin.project_path`; for user-scope installs with a CWD override, from the current working directory. The User row is never annotated because its source is always `~/.claude/settings.json`. The CLI's `ccpm info` command renders the same suffix in plain text for parity.
```

If the architecture doc has no dedicated TUI/details section, add the paragraph under the most relevant existing section (likely "UI" or "Rendering"). Do not invent new top-level sections.

- [ ] **Step 5: Verify documentation builds (no broken Markdown)**

Run:
```bash
git diff --stat CLAUDE.md CLAUDE.local.md FEATURE_PLAN.md docs/architecture.md
```
Expected: only the four files in the diff, with reasonable line counts (a few additions each, no deletions).

- [ ] **Step 6: Commit**

Run:
```bash
git add CLAUDE.md CLAUDE.local.md FEATURE_PLAN.md docs/architecture.md
git commit -m "Document plugin override-source visibility feature"
```

---

## Task 7: Final verification

**Files:**
- None modified

**Goal:** Confirm the whole change set is green before handoff. No push — the user pushes when ready.

- [ ] **Step 1: Run full test suite**

Run:
```bash
cargo test
```
Expected: all tests pass. Verify the unit-test count grew by exactly 9 (Tasks 2 + 3 added 6 + 3) and the `details::tests` module added 3 tests, and `tests/integration.rs` has 1 new test.

- [ ] **Step 2: Run lint**

Run:
```bash
cargo clippy -- -D warnings
```
Expected: zero warnings.

- [ ] **Step 3: Run formatter check**

Run:
```bash
cargo fmt --check
```
Expected: passes silently. If it reports diffs, run `cargo fmt`, re-run tests, and amend the relevant commit (only if the formatting fix lives entirely within files touched by that commit; otherwise add a separate "rustfmt" commit).

- [ ] **Step 4: Confirm branch state**

Run:
```bash
git log --oneline feature/plugin-override-source-visibility
```
Expected: a clean history of commits matching:
1. `Add design spec for plugin override-source visibility` (Task 0 — already there)
2. `Add Plugin::project_settings_source helper` (Task 2)
3. `Add Plugin::project_settings_source_display with home-relative format` (Task 3)
4. `Show override source file path on Project/Local Settings rows in TUI` (Task 4)
5. `Show override source file path in ccpm info Settings block` (Task 5)
6. `Document plugin override-source visibility feature` (Task 6)

- [ ] **Step 5: Manual smoke test**

Run from `/Users/kaldown/Projects/ccpm` (or any project that lives outside Ternv3):
```bash
cargo run --quiet -- info chrome-devtools-mcp@claude-plugins-official
```
Expected: output contains `Settings:` block. If the user has Ternv3 still pinning chrome-devtools-mcp (verify with `cat ~/Projects/tern/Ternv3/.claude/settings.local.json`), the Local row should be `(no setting)` because we're NOT in Ternv3 — that flag is in Ternv3's local file, not this CWD's. This confirms the CWD-relative semantics work correctly.

If the user wants to see the annotation: `cd ~/Projects/tern/Ternv3 && cargo run --manifest-path ~/Projects/ccpm/Cargo.toml --quiet -- info chrome-devtools-mcp@claude-plugins-official` — Local row should now show `· ~/Projects/tern/Ternv3/.claude/settings.local.json`.

- [ ] **Step 6: Report completion**

Report to the user: "Plan complete. All 6 commits on `feature/plugin-override-source-visibility`, all tests pass, clippy clean, fmt clean. Ready to push or open PR when you're ready."

Do **not** push — branch push is the user's call.
