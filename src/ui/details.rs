use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(plugin) = app.selected_plugin() {
        let mut lines = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&plugin.name),
            ]),
            Line::from(vec![
                Span::styled(
                    "Marketplace: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(&plugin.marketplace),
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

        // Installation scope (where it's installed)
        let install_location = match (plugin.install_scope, plugin.is_current_project) {
            (crate::plugin::Scope::User, _) => "User (~/.claude)".to_string(),
            (crate::plugin::Scope::Project, true) => "Project (this project)".to_string(),
            (crate::plugin::Scope::Project, false) => "Project (other project)".to_string(),
            (crate::plugin::Scope::Local, true) => "Local (this project)".to_string(),
            (crate::plugin::Scope::Local, false) => "Local (other project)".to_string(),
        };
        lines.push(Line::from(vec![
            Span::styled("Installed: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(install_location),
        ]));

        // Enabled context (where it's enabled)
        lines.push(Line::from(vec![
            Span::styled(
                "Enabled in: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(plugin.enabled_context()),
        ]));

        // Always show project path for project/local scopes (using relative-to-home format)
        if plugin.install_scope != crate::plugin::Scope::User {
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
                Span::raw(version),
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
                Span::raw(date),
            ]));
        }

        // Description section
        lines.push(Line::from(""));
        if let Some(ref description) = plugin.description {
            lines.push(Line::from(vec![Span::styled(
                "Description:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(Span::raw(description)));
        }

        lines
    } else {
        vec![Line::from(Span::styled(
            "No plugin selected",
            Style::default().fg(Color::DarkGray),
        ))]
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
