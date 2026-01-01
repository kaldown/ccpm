mod detail_modal;
mod details;
mod dialogs;
mod help;
mod plugin_list;

pub use detail_modal::render_detail_modal;
pub use details::render_details;
pub use dialogs::render_confirm_dialog;
pub use help::render_help;
pub use plugin_list::render_plugin_list;

use crate::app::{App, AppMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Main layout: header, content, footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(10),   // Content
            Constraint::Length(3), // Footer
        ])
        .split(area);

    // Render header
    render_header(frame, app, main_chunks[0]);

    // Content area: split into left (list) and right (details)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    // Render plugin list
    render_plugin_list(frame, app, content_chunks[0]);

    // Render details panel
    render_details(frame, app, content_chunks[1]);

    // Render footer/command bar
    render_footer(frame, app, main_chunks[2]);

    // Render overlays based on mode
    match app.mode {
        AppMode::Help => render_help(frame, area),
        AppMode::Confirm(action) => render_confirm_dialog(frame, app, action, area),
        AppMode::DetailModal => render_detail_modal(frame, app, area),
        _ => {}
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let (enabled, total) = app.plugin_count();

    let title = vec![
        Span::styled(
            " CCPM ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("│ "),
        Span::styled(
            format!("Scope: {} ", app.scope_filter.label()),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("│ "),
        Span::styled(
            format!("{}/{} enabled ", enabled, total),
            Style::default().fg(Color::Green),
        ),
    ];

    // Add search indicator if in search mode
    let title = if app.mode == AppMode::Search || !app.search_query.is_empty() {
        let mut t = title;
        t.push(Span::raw("│ "));
        t.push(Span::styled(
            format!("Search: {}", app.search_query),
            Style::default().fg(Color::Magenta),
        ));
        if app.mode == AppMode::Search {
            t.push(Span::styled("_", Style::default().fg(Color::Magenta)));
        }
        t
    } else {
        title
    };

    let header = Paragraph::new(Line::from(title)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(header, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let commands = match app.mode {
        AppMode::Normal => vec![
            ("j/k", "navigate"),
            ("Enter", "details"),
            ("e", "enable"),
            ("d", "disable"),
            ("s", "scope"),
            ("/", "search"),
            ("?", "help"),
            ("q", "quit"),
        ],
        AppMode::Search => vec![("Enter/Esc", "exit search"), ("Type", "filter")],
        AppMode::Help => vec![("Esc/?", "close help")],
        AppMode::Confirm(_) => vec![("y", "confirm"), ("n/Esc", "cancel")],
        AppMode::DetailModal => vec![("Esc/Enter", "close"), ("Space", "toggle")],
    };

    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in commands.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw(" │ "));
        }
        spans.push(Span::styled(
            format!(" {} ", key),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(format!(" {} ", desc)));
    }

    // Add status message if present
    let footer_content = if let Some(ref msg) = app.message {
        let color = if msg.is_error {
            Color::Red
        } else {
            Color::Green
        };
        vec![
            Line::from(spans),
            Line::from(Span::styled(&msg.text, Style::default().fg(color))),
        ]
    } else {
        vec![Line::from(spans)]
    };

    let footer = Paragraph::new(footer_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(footer, area);
}
