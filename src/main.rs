mod tui;
use chrono::NaiveDate;
use clap::{Parser, Subcommand};

use property_manager::db;
use property_manager::db::reporting::*;
use property_manager::db::repository::*;
use property_manager::error::AppResult;
use property_manager::models::expense::Expense;
use property_manager::models::lease::Lease;
use property_manager::models::property::Property;
use property_manager::models::rent_payment::RentPayment;
use property_manager::models::tenant::Tenant;
use rusqlite::Connection;

#[derive(Parser)]
#[command(name = "property-manager", about = "Gestion de biens immobiliers")]
struct Cli {
    /// Chemin vers le fichier de base de données SQLite
    #[arg(long, default_value = "property_manager.db")]
    db_path: String,

    #[command(subcommand)]
    command: Command,
}

struct IndirectExpenseArgs {
    category: String,
    amount: f64,
    date: String,
    properties: Vec<i64>,
    recurring: bool,
}

struct AddPropertyInput {
    label: String,
    address: String,
    purchase_date: String,
    purchase_price: f64,
    notes: Option<String>,
}

struct AddExpenseInput {
    property_id: i64,
    category: String,
    amount: f64,
    date: String,
    recurring: bool,
}

impl IndirectExpenseArgs {
    fn new(
        category: String,
        amount: f64,
        date: String,
        properties: Vec<i64>,
        recurring: bool,
    ) -> Self {
        Self {
            category,
            amount,
            date,
            properties,
            recurring,
        }
    }
}

impl AddPropertyInput {
    fn new(
        label: String,
        address: String,
        purchase_date: String,
        purchase_price: f64,
        notes: Option<String>,
    ) -> Self {
        Self {
            label,
            address,
            purchase_date,
            purchase_price,
            notes,
        }
    }
}

impl AddExpenseInput {
    fn new(property_id: i64, category: String, amount: f64, date: String, recurring: bool) -> Self {
        Self {
            property_id,
            category,
            amount,
            date,
            recurring,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Enregistrer un frais indirect, réparti à parts égales entre plusieurs biens
    AddIndirectExpense {
        category: String,
        /// Montant total en euros
        amount: f64,
        /// Date au format YYYY-MM-DD
        date: String,
        /// Ids des biens concernés, séparés par des virgules (ex. 1,2,3)
        #[arg(long, value_delimiter = ',')]
        properties: Vec<i64>,
        #[arg(long)]
        recurring: bool,
    },

    /// Afficher le dashboard interactif dans le terminal
    Dashboard,

    /// Lister tous les baux actifs
    ListActiveLeases,

    /// Lister les dépenses d'un bien
    ListExpenses { property_id: i64 },

    /// Supprimer un bien (refusé s'il a des baux ou dépenses rattachés)
    DeleteProperty { property_id: i64 },

    /// Enregistrer un nouveau bien
    AddProperty {
        label: String,
        address: String,
        /// Date d'achat au format YYYY-MM-DD
        purchase_date: String,
        /// Prix d'achat en euros (converti automatiquement en centimes)
        purchase_price: f64,
        #[arg(long)]
        notes: Option<String>,
    },

    /// Lister tous les biens enregistrés
    ListProperties,

    /// Enregistrer un locataire
    AddTenant {
        name: String,
        #[arg(long)]
        contact: Option<String>,
    },

    /// Créer un bail pour un bien
    AddLease {
        property_id: i64,
        tenant_id: i64,
        /// Loyer mensuel en euros
        monthly_rent: f64,
        /// Date de début au format YYYY-MM-DD
        start_date: String,
    },

    /// Enregistrer une dépense pour un bien
    AddExpense {
        property_id: i64,
        category: String,
        /// Montant en euros
        amount: f64,
        /// Date au format YYYY-MM-DD
        date: String,
        #[arg(long)]
        recurring: bool,
    },

    /// Enregistrer un paiement de loyer pour un bail
    AddPayment {
        lease_id: i64,
        /// Montant en euros
        amount: f64,
        /// Date au format YYYY-MM-DD
        date: String,
        /// Période concernée, format YYYY-MM
        period: String,
    },

    /// Afficher la rentabilité de tous les biens
    Profitability,

    /// Afficher les baux avec des loyers en retard
    Overdue {
        /// Date de référence pour le calcul, format YYYY-MM-DD (par défaut : aujourd'hui)
        #[arg(long)]
        up_to: Option<String>,
    },
}

fn parse_date(s: &str) -> AppResult<NaiveDate> {
    Ok(NaiveDate::parse_from_str(s, "%Y-%m-%d")?)
}

/// Convertit un montant en euros (saisi par l'utilisateur) en centimes.
fn euros_to_cents(euros: f64) -> i64 {
    (euros * 100.0).round() as i64
}

fn main() -> AppResult<()> {
    let cli = Cli::parse();
    let conn = db::open(&cli.db_path)?;

    // Cas particulier : le dashboard gère sa propre connexion en interne
    // et prend le contrôle du terminal, donc on le sort du dispatch commun.
    if let Command::Dashboard = cli.command {
        drop(conn);
        tui::run(&cli.db_path)?;
        return Ok(());
    }

    run_command(&conn, cli.command)
}

fn run_command(conn: &Connection, command: Command) -> AppResult<()> {
    match command {
        Command::Dashboard => unreachable!("géré en amont dans main()"),

        Command::ListActiveLeases => handle_list_active_leases(conn)?,

        Command::ListExpenses { property_id } => handle_list_expenses(conn, property_id)?,

        Command::DeleteProperty { property_id } => handle_delete_property(conn, property_id),

        Command::ListProperties => handle_list_properties(conn)?,

        Command::AddTenant { name, contact } => handle_add_tenant(conn, name, contact)?,

        Command::AddLease {
            property_id,
            tenant_id,
            monthly_rent,
            start_date,
        } => handle_add_lease(conn, property_id, tenant_id, monthly_rent, start_date)?,

        Command::AddPayment {
            lease_id,
            amount,
            date,
            period,
        } => handle_add_payment(conn, lease_id, amount, date, period)?,

        Command::Profitability => handle_profitability(conn)?,

        Command::Overdue { up_to } => handle_overdue(conn, up_to)?,

        other => run_add_command(conn, other)?,
    }
    Ok(())
}

fn run_add_command(conn: &Connection, command: Command) -> AppResult<()> {
    match command {
        Command::AddIndirectExpense {
            category,
            amount,
            date,
            properties,
            recurring,
        } => handle_add_indirect_expense(
            conn,
            IndirectExpenseArgs::new(category, amount, date, properties, recurring),
        ),

        Command::AddProperty {
            label,
            address,
            purchase_date,
            purchase_price,
            notes,
        } => handle_add_property(
            conn,
            AddPropertyInput::new(label, address, purchase_date, purchase_price, notes),
        ),

        Command::AddExpense {
            property_id,
            category,
            amount,
            date,
            recurring,
        } => handle_add_expense(
            conn,
            AddExpenseInput::new(property_id, category, amount, date, recurring),
        ),

        _ => unreachable!("géré en amont dans run_command()"),
    }
}

fn handle_add_indirect_expense(conn: &Connection, input: IndirectExpenseArgs) -> AppResult<()> {
    let count = input.properties.len();
    let id = insert_indirect_expense(
        conn,
        &IndirectExpenseInput {
            category: input.category,
            total_amount_cents: euros_to_cents(input.amount),
            expense_date: parse_date(&input.date)?,
            recurring: input.recurring,
            property_ids: input.properties,
        },
    )?;
    println!(
        "Frais indirect enregistré (id {}), réparti sur {} bien(s)",
        id, count
    );
    Ok(())
}

fn handle_list_active_leases(conn: &Connection) -> AppResult<()> {
    let leases = list_active_leases(conn)?;
    if leases.is_empty() {
        println!("Aucun bail actif.");
    }
    for l in leases {
        println!(
            "[bail {}] bien #{} — locataire #{} — loyer {:.2} €/mois — depuis le {}",
            l.id.unwrap(),
            l.property_id,
            l.tenant_id,
            l.monthly_rent_cents as f64 / 100.0,
            l.start_date
        );
    }
    Ok(())
}

fn handle_list_expenses(conn: &Connection, property_id: i64) -> AppResult<()> {
    let expenses = list_expense_lines_for_property(conn, property_id)?;
    if expenses.is_empty() {
        println!("Aucune dépense enregistrée pour ce bien.");
    }
    for e in expenses {
        let tag = if e.is_indirect { " [indirect]" } else { "" };
        println!(
            "{} — {:.2} € — {}{}{}",
            e.category,
            e.allocated_amount_cents as f64 / 100.0,
            e.expense_date,
            if e.recurring { " (récurrente)" } else { "" },
            tag
        );
    }
    Ok(())
}

fn handle_delete_property(conn: &Connection, property_id: i64) {
    match delete_property(conn, property_id) {
        Ok(()) => println!("Bien {} supprimé.", property_id),
        Err(e) => println!("Suppression refusée : {}", e),
    }
}

fn handle_add_property(conn: &Connection, input: AddPropertyInput) -> AppResult<()> {
    let property = Property::new(
        input.label,
        input.address,
        parse_date(&input.purchase_date)?,
        euros_to_cents(input.purchase_price),
        input.notes,
    )?;
    let id = insert_property(conn, &property)?;
    println!("Bien créé avec l'id {}", id);
    Ok(())
}

fn handle_list_properties(conn: &Connection) -> AppResult<()> {
    let properties = list_properties(conn)?;
    if properties.is_empty() {
        println!("Aucun bien enregistré.");
    }
    for p in properties {
        println!(
            "[{}] {} — {} — acheté le {} pour {:.2} €",
            p.id().unwrap(),
            p.label(),
            p.address(),
            p.purchase_date(),
            p.purchase_price_cents() as f64 / 100.0
        );
    }
    Ok(())
}

fn handle_add_tenant(conn: &Connection, name: String, contact: Option<String>) -> AppResult<()> {
    let tenant = Tenant::new(name, contact);
    let id = insert_tenant(conn, &tenant)?;
    println!("Locataire créé avec l'id {}", id);
    Ok(())
}

fn handle_add_lease(
    conn: &Connection,
    property_id: i64,
    tenant_id: i64,
    monthly_rent: f64,
    start_date: String,
) -> AppResult<()> {
    let lease = Lease::new(
        property_id,
        tenant_id,
        euros_to_cents(monthly_rent),
        parse_date(&start_date)?,
        None,
    );
    let id = insert_lease(conn, &lease)?;
    println!("Bail créé avec l'id {}", id);
    Ok(())
}

fn handle_add_expense(conn: &Connection, input: AddExpenseInput) -> AppResult<()> {
    let expense = Expense::new_direct(
        input.property_id,
        input.category,
        euros_to_cents(input.amount),
        parse_date(&input.date)?,
        input.recurring,
    )?;
    let id = insert_expense(conn, &expense)?;
    println!("Dépense enregistrée avec l'id {}", id);
    Ok(())
}

fn handle_add_payment(
    conn: &Connection,
    lease_id: i64,
    amount: f64,
    date: String,
    period: String,
) -> AppResult<()> {
    let payment = RentPayment::new(lease_id, euros_to_cents(amount), parse_date(&date)?, period);
    let id = insert_rent_payment(conn, &payment)?;
    println!("Paiement enregistré avec l'id {}", id);
    Ok(())
}

fn handle_profitability(conn: &Connection) -> AppResult<()> {
    let results = all_properties_profitability(conn)?;
    if results.is_empty() {
        println!("Aucun bien enregistré.");
    }
    for r in results {
        println!(
            "{:<20} loyers: {:>10.2} €   dépenses: {:>10.2} €   net: {:>10.2} €",
            r.label,
            r.total_rent_collected as f64 / 100.0,
            r.total_expenses as f64 / 100.0,
            r.net_result as f64 / 100.0
        );
    }
    Ok(())
}

fn handle_overdue(conn: &Connection, up_to: Option<String>) -> AppResult<()> {
    let reference_date = match up_to {
        Some(s) => parse_date(&s)?,
        None => chrono::Local::now().date_naive(),
    };
    let overdue = all_overdue_leases(conn, reference_date)?;
    if overdue.is_empty() {
        println!("Aucun loyer en retard.");
    }
    for o in overdue {
        println!(
            "{} ({}) — mois manquants: {}",
            o.property_label,
            o.tenant_name,
            o.missing_months.join(", ")
        );
    }
    Ok(())
}
