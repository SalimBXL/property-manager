use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
};

use property_manager::db::reporting::{OverdueLease, PropertyDetail, PropertyProfitability};

pub fn draw(
    frame: &mut Frame,
    tab_labels: &[String],
    selected_tab: usize,
    overview: (&[PropertyProfitability], &[OverdueLease]),
    detail: Option<&PropertyDetail>,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // titre
            Constraint::Length(3), // onglets
            Constraint::Min(0),    // contenu
        ])
        .split(frame.area());

    draw_title(frame, chunks[0]);
    draw_tabs(frame, chunks[1], tab_labels, selected_tab);

    if selected_tab == 0 {
        draw_overview(frame, chunks[2], overview.0, overview.1);
    } else if let Some(detail) = detail {
        draw_property_detail(frame, chunks[2], detail);
    }
}

fn draw_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            "Property Manager",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("  —  ←/→ changer d'onglet  —  'r' rafraîchir  —  'q' quitter"),
    ]))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, area);
}

fn draw_tabs(frame: &mut Frame, area: Rect, labels: &[String], selected: usize) {
    let titles: Vec<Line> = labels.iter().map(|l| Line::from(l.clone())).collect();
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL))
        .select(selected)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn draw_overview(
    frame: &mut Frame,
    area: Rect,
    profitability: &[PropertyProfitability],
    overdue: &[OverdueLease],
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let header = Row::new(vec!["Bien", "Loyers encaissés", "Dépenses", "Net"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = profitability
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
    frame.render_widget(table, chunks[0]);

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
    frame.render_widget(panel, chunks[1]);
}

fn draw_property_detail(frame: &mut Frame, area: Rect, detail: &PropertyDetail) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // résumé
            Constraint::Min(0),    // dépenses
        ])
        .split(area);

    let net_style = if detail.net_result >= 0 {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    };

    let mut summary_lines = vec![
        Line::from(format!(
            "{} — {}",
            detail.property.label, detail.property.address
        )),
        Line::from(format!(
            "Loyers encaissés : {:.2} €   Dépenses : {:.2} €   Net : ",
            detail.total_rent_collected as f64 / 100.0,
            detail.total_expenses as f64 / 100.0,
        ))
        .style(Style::default()), // le net est ajouté à part pour le colorer
    ];
    summary_lines.push(Line::from(Span::styled(
        format!("Net : {:.2} €", detail.net_result as f64 / 100.0),
        net_style,
    )));

    if detail.missing_months.is_empty() {
        summary_lines.push(Line::from(Span::styled(
            "Loyers à jour.",
            Style::default().fg(Color::Green),
        )));
    } else {
        summary_lines.push(Line::from(Span::styled(
            format!("Mois impayés : {}", detail.missing_months.join(", ")),
            Style::default().fg(Color::Red),
        )));
    }

    let summary = Paragraph::new(summary_lines)
        .block(Block::default().borders(Borders::ALL).title(" Résumé "));
    frame.render_widget(summary, chunks[0]);

    let header = Row::new(vec!["Catégorie", "Montant", "Date", "Récurrente", "Type"])
        .style(Style::default().add_modifier(Modifier::BOLD));
    let rows: Vec<Row> = detail
        .expenses
        .iter()
        .map(|e| {
            let amount_display = if e.is_indirect {
                format!(
                    "{:.2} € (sur {:.2} €)",
                    e.allocated_amount_cents as f64 / 100.0,
                    e.total_amount_cents as f64 / 100.0
                )
            } else {
                format!("{:.2} €", e.allocated_amount_cents as f64 / 100.0)
            };
            let type_style = if e.is_indirect {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(e.category.clone()),
                Cell::from(amount_display),
                Cell::from(e.expense_date.to_string()),
                Cell::from(if e.recurring { "oui" } else { "non" }),
                Cell::from(if e.is_indirect { "indirect" } else { "direct" }).style(type_style),
            ])
        })
        .collect();
    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" Dépenses "));
    frame.render_widget(table, chunks[1]);
}
