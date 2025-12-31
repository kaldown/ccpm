use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_help(frame: &mut Frame, area: Rect) {
    // Center the help dialog
    let help_area = centered_rect(60, 70, area);

    // Clear the background
    frame.render_widget(Clear, help_area);

    let keybindings = vec![
        (
            "Navigation",
            vec![
                ("j / ↓", "Move down"),
                ("k / ↑", "Move up"),
                ("g", "Go to first"),
                ("G", "Go to last"),
                ("Enter", "View details"),
            ],
        ),
        (
            "Plugin Actions",
            vec![
                ("e", "Enable plugin"),
                ("d", "Disable plugin"),
                ("Space", "Toggle enable/disable"),
                ("u", "Toggle auto-update"),
                ("x", "Remove plugin"),
                ("U", "Update plugin"),
            ],
        ),
        (
            "Filtering",
            vec![
                ("s", "Cycle scope filter (All/User/Local)"),
                ("/", "Start search"),
                ("Esc", "Clear search / Exit mode"),
            ],
        ),
        (
            "General",
            vec![("?", "Toggle help"), ("r", "Reload plugins"), ("q", "Quit")],
        ),
    ];

    let mut lines = vec![
        Line::from(Span::styled(
            "CCPM - Claude Code Plugin Manager",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for (section, bindings) in keybindings {
        lines.push(Line::from(Span::styled(
            section,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));

        for (key, desc) in bindings {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {:12}", key),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(desc),
            ]));
        }

        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "Press ? or Esc to close",
        Style::default().fg(Color::DarkGray),
    )));

    let help = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Help ")
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Left);

    frame.render_widget(help, help_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
