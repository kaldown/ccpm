use ccpm::app::{App, AppMode};
use ccpm::cli::{run_command, Cli};
use ccpm::ui;
use clap::Parser;
use color_eyre::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => run_command(cmd),
        None => run_tui(),
    }
}

fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Main loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, app))?;

        if let Event::Key(key) = event::read()? {
            // Handle Ctrl+C globally
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.quit();
            }

            match app.mode {
                AppMode::Normal => handle_normal_mode(app, key.code),
                AppMode::Search => handle_search_mode(app, key.code),
                AppMode::Help => handle_help_mode(app, key.code),
                AppMode::Confirm(_) => handle_confirm_mode(app, key.code),
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_normal_mode(app: &mut App, key: KeyCode) {
    match key {
        // Navigation
        KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
        KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
        KeyCode::Char('g') => app.select_first(),
        KeyCode::Char('G') => app.select_last(),

        // Plugin actions
        KeyCode::Char('e') => app.enable_selected_plugin(),
        KeyCode::Char('d') => app.disable_selected_plugin(),
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_selected_plugin(),
        KeyCode::Char('x') => app.confirm_remove(),

        // Filtering
        KeyCode::Char('s') => app.cycle_scope_filter(),
        KeyCode::Char('/') => app.start_search(),

        // Reload
        KeyCode::Char('r') => {
            if let Err(e) = app.reload_plugins() {
                app.message = Some(ccpm::app::StatusMessage::error(format!(
                    "Reload failed: {}",
                    e
                )));
            } else {
                app.message = Some(ccpm::app::StatusMessage::info("Plugins reloaded"));
            }
        }

        // Help and quit
        KeyCode::Char('?') => app.show_help(),
        KeyCode::Char('q') => app.quit(),
        KeyCode::Esc => app.clear_search(),

        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Enter => app.end_search(),
        KeyCode::Backspace => app.delete_search_char(),
        KeyCode::Char(c) => app.append_search_char(c),
        _ => {}
    }
}

fn handle_help_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => app.hide_help(),
        _ => {}
    }
}

fn handle_confirm_mode(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Enter => app.execute_confirm(),
        KeyCode::Char('n') | KeyCode::Esc => app.cancel_confirm(),
        _ => {}
    }
}
