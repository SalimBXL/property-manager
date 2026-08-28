use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use property_manager::db;
use property_manager::db::reporting::{
    PropertyDetail, all_overdue_leases, all_properties_profitability, property_detail,
};
use property_manager::db::repository::list_properties;
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

    let mut properties = list_properties(conn)?;
    let mut profitability = all_properties_profitability(conn)?;
    let mut overdue = all_overdue_leases(conn, today)?;
    let mut selected_tab: usize = 0;

    loop {
        let tab_count = properties.len() + 1; // +1 pour "Vue d'ensemble"
        let mut tab_labels = vec!["Vue d'ensemble".to_string()];
        tab_labels.extend(properties.iter().map(|p| p.label.clone()));

        let detail: Option<PropertyDetail> = if selected_tab > 0 {
            let property_id = properties[selected_tab - 1].id.unwrap();
            Some(property_detail(conn, property_id, today)?)
        } else {
            None
        };

        terminal.draw(|frame| {
            ui::draw(
                frame,
                &tab_labels,
                selected_tab,
                (&profitability, &overdue),
                detail.as_ref(),
            )
        })?;

        if event::poll(Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            match key.code {
                KeyCode::Char('q') => return Ok(()),
                KeyCode::Char('r') => {
                    properties = list_properties(conn)?;
                    profitability = all_properties_profitability(conn)?;
                    overdue = all_overdue_leases(conn, today)?;
                    if selected_tab >= tab_count {
                        selected_tab = 0; // le bien affiché a peut-être été supprimé
                    }
                }
                KeyCode::Right => {
                    selected_tab = (selected_tab + 1) % tab_count;
                }
                KeyCode::Left => {
                    selected_tab = if selected_tab == 0 {
                        tab_count - 1
                    } else {
                        selected_tab - 1
                    };
                }
                _ => {}
            }
        }
    }
}
