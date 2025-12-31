use crate::app::{App, ConfirmAction};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render_confirm_dialog(frame: &mut Frame, app: &App, action: ConfirmAction, area: Rect) {
    let dialog_area = centered_rect(50, 30, area);

    // Clear the background
    frame.render_widget(Clear, dialog_area);

    let (title, message) = match action {
        ConfirmAction::Remove => {
            let plugin_name = app
                .selected_plugin()
                .map(|p| p.display_name())
                .unwrap_or_else(|| "unknown".to_string());
            (
                " Confirm Remove ",
                format!("Are you sure you want to remove '{}'?", plugin_name),
            )
        }
    };

    let content = vec![
        Line::from(""),
        Line::from(Span::raw(&message)),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " y ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" Yes  "),
            Span::styled(
                " n ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" No"),
        ]),
    ];

    let dialog = Paragraph::new(content)
        .block(
            Block::default()
                .title(title)
                .title_alignment(Alignment::Center)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(dialog, dialog_area);
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
