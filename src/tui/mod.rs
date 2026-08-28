use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use chrono::NaiveDate;
use rusqlite::Connection;

use property_manager::db;
use property_manager::db::reporting::{
    OverdueLease, PropertyDetail, PropertyProfitability, all_overdue_leases,
    all_properties_profitability, property_detail,
};
use property_manager::db::repository::list_properties;
use property_manager::error::{AppError, AppResult};
use property_manager::models::property::Property;

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

/// État mutable du dashboard : données chargées depuis la base + onglet
/// actuellement sélectionné.
struct DashboardState {
    properties: Vec<Property>,
    profitability: Vec<PropertyProfitability>,
    overdue: Vec<OverdueLease>,
    selected_tab: usize,
}

impl DashboardState {
    fn load(conn: &Connection, today: NaiveDate) -> AppResult<Self> {
        Ok(Self {
            properties: list_properties(conn)?,
            profitability: all_properties_profitability(conn)?,
            overdue: all_overdue_leases(conn, today)?,
            selected_tab: 0,
        })
    }

    fn refresh(&mut self, conn: &Connection, today: NaiveDate) -> AppResult<()> {
        self.properties = list_properties(conn)?;
        self.profitability = all_properties_profitability(conn)?;
        self.overdue = all_overdue_leases(conn, today)?;
        // le bien affiché a peut-être été supprimé
        if self.selected_tab >= self.tab_count() {
            self.selected_tab = 0;
        }
        Ok(())
    }

    fn tab_count(&self) -> usize {
        self.properties.len() + 1 // +1 pour "Vue d'ensemble"
    }

    fn tab_labels(&self) -> Vec<String> {
        let mut labels = vec!["Vue d'ensemble".to_string()];
        labels.extend(self.properties.iter().map(|p| p.label.clone()));
        labels
    }

    fn detail(&self, conn: &Connection, today: NaiveDate) -> AppResult<Option<PropertyDetail>> {
        if self.selected_tab == 0 {
            return Ok(None);
        }
        let property_id = self.properties[self.selected_tab - 1].id.ok_or_else(|| {
            AppError::Internal("un bien lu depuis la base doit toujours avoir un id".to_string())
        })?;
        Ok(Some(property_detail(conn, property_id, today)?))
    }

    fn move_right(&mut self) {
        self.selected_tab = (self.selected_tab + 1) % self.tab_count();
    }

    fn move_left(&mut self) {
        let count = self.tab_count();
        self.selected_tab = if self.selected_tab == 0 {
            count - 1
        } else {
            self.selected_tab - 1
        };
    }
}

enum LoopAction {
    Continue,
    Quit,
}

fn handle_key(
    code: KeyCode,
    state: &mut DashboardState,
    conn: &Connection,
    today: NaiveDate,
) -> AppResult<LoopAction> {
    match code {
        KeyCode::Char('q') => Ok(LoopAction::Quit),
        KeyCode::Char('r') => {
            state.refresh(conn, today)?;
            Ok(LoopAction::Continue)
        }
        KeyCode::Right => {
            state.move_right();
            Ok(LoopAction::Continue)
        }
        KeyCode::Left => {
            state.move_left();
            Ok(LoopAction::Continue)
        }
        _ => Ok(LoopAction::Continue),
    }
}

fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    conn: &Connection,
) -> AppResult<()> {
    let today = chrono::Local::now().date_naive();
    let mut state = DashboardState::load(conn, today)?;

    loop {
        let tab_labels = state.tab_labels();
        let detail = state.detail(conn, today)?;

        terminal.draw(|frame| {
            ui::draw(
                frame,
                &tab_labels,
                state.selected_tab,
                (&state.profitability, &state.overdue),
                detail.as_ref(),
            )
        })?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };

        if let LoopAction::Quit = handle_key(key.code, &mut state, conn, today)? {
            return Ok(());
        }
    }
}
