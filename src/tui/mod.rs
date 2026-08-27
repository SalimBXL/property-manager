use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use property_manager::db;
use property_manager::db::reporting::{all_overdue_leases, all_properties_profitability};
use property_manager::error::AppResult;

mod ui;

pub fn run(db_path: &str) -> AppResult<()> {
    let conn = db::open(db_path)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = event_loop(&mut terminal, &conn);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    conn: &rusqlite::Connection,
) -> AppResult<()> {
    let today = chrono::Local::now().date_naive();

    let mut profitability = all_properties_profitability(conn)?;
    let mut overdue = all_overdue_leases(conn, today)?;

    loop {
        terminal.draw(|frame| ui::draw(frame, &profitability, &overdue))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('r') => {
                        profitability = all_properties_profitability(conn)?;
                        overdue = all_overdue_leases(conn, today)?;
                    }
                    _ => {}
                }
            }
        }
    }
}
