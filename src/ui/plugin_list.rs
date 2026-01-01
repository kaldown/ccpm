use crate::app::App;
use crate::plugin::Scope;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub fn render_plugin_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .filtered_plugins
        .iter()
        .map(|&idx| {
            let plugin = &app.plugins[idx];

            // Scope indicator: [U], [L], or [L*] for local in other project
            let scope_indicator = Span::styled(
                plugin.scope_indicator(),
                match (plugin.install_scope, plugin.is_current_project) {
                    (Scope::User, _) => Style::default().fg(Color::Blue),
                    (Scope::Local, true) => Style::default().fg(Color::Magenta),
                    (Scope::Local, false) => Style::default().fg(Color::Yellow), // Different project
                },
            );

            let status_indicator = if plugin.is_enabled() {
                Span::styled(" [+] ", Style::default().fg(Color::Green))
            } else {
                Span::styled(" [-] ", Style::default().fg(Color::DarkGray))
            };

            let name_style = if plugin.is_enabled() {
                Style::default().fg(Color::White)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let name = Span::styled(&plugin.name, name_style);

            let marketplace = Span::styled(
                format!(" @{}", plugin.marketplace),
                Style::default().fg(Color::DarkGray),
            );

            ListItem::new(Line::from(vec![
                scope_indicator,
                status_indicator,
                name,
                marketplace,
            ]))
        })
        .collect();

    let title = format!(" Plugins ({}) ", app.filtered_plugins.len());

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_index));

    frame.render_stateful_widget(list, area, &mut state);
}
