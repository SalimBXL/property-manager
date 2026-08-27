use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

use property_manager::db::reporting::{OverdueLease, PropertyProfitability};

pub fn draw(frame: &mut Frame, profitability: &[PropertyProfitability], overdue: &[OverdueLease]) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // titre
            Constraint::Percentage(60), // tableau rentabilité
            Constraint::Percentage(40), // loyers en retard
        ])
        .split(frame.area());

    draw_title(frame, chunks[0]);
    draw_profitability_table(frame, chunks[1], profitability);
    draw_overdue_panel(frame, chunks[2], overdue);
}

fn draw_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "Property Manager",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("  —  'r' rafraîchir  —  'q' quitter"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, area);
}

fn draw_profitability_table(frame: &mut Frame, area: Rect, data: &[PropertyProfitability]) {
    let header = Row::new(vec!["Bien", "Loyers encaissés", "Dépenses", "Net"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = data
        .iter()
        .map(|p| {
            let net_style = if p.net_result >= 0 {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Red)
            };
            Row::new(vec![
                Cell::from(p.label.clone()),
                Cell::from(format!("{:.2} €", p.total_rent_collected as f64 / 100.0)),
                Cell::from(format!("{:.2} €", p.total_expenses as f64 / 100.0)),
                Cell::from(format!("{:.2} €", p.net_result as f64 / 100.0)).style(net_style),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(40),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
        Constraint::Percentage(20),
    ];

    let table = Table::new(rows, widths).header(header).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Rentabilité par bien "),
    );

    frame.render_widget(table, area);
}

fn draw_overdue_panel(frame: &mut Frame, area: Rect, overdue: &[OverdueLease]) {
    let lines: Vec<Line> = if overdue.is_empty() {
        vec![Line::from(Span::styled(
            "Aucun loyer en retard.",
            Style::default().fg(Color::Green),
        ))]
    } else {
        overdue
            .iter()
            .map(|o| {
                Line::from(Span::styled(
                    format!(
                        "{} ({}) — mois manquants : {}",
                        o.property_label,
                        o.tenant_name,
                        o.missing_months.join(", ")
                    ),
                    Style::default().fg(Color::Red),
                ))
            })
            .collect()
    };

    let panel = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Loyers en retard "),
    );

    frame.render_widget(panel, area);
}
