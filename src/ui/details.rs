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

fn format_setting(value: Option<bool>) -> Span<'static> {
    match value {
        Some(true) => Span::styled("enabled", Style::default().fg(Color::Green)),
        Some(false) => Span::styled("disabled", Style::default().fg(Color::Red)),
        None => Span::styled("(no setting)", Style::default().fg(Color::DarkGray)),
    }
}

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

pub fn build_details_lines(plugin: &Plugin, cwd: &Path) -> Vec<Line<'static>> {
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

    // Project row — annotated with source file path when set
    {
        let mut spans = vec![
            Span::raw("  Project: "),
            format_setting(plugin.enabled_project),
        ];
        if let Some(path) = plugin.project_settings_source_display(Scope::Project, cwd) {
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
        if let Some(path) = plugin.project_settings_source_display(Scope::Local, cwd) {
            spans.push(Span::styled(
                format!("  · {}", path),
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(spans));
    }

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
