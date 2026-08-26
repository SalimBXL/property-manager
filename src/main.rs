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

#[derive(Subcommand)]
enum Command {
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
    match cli.command {
        Command::AddProperty {
            label,
            address,
            purchase_date,
            purchase_price,
            notes,
        } => {
            let property = Property::new(
                label,
                address,
                parse_date(&purchase_date)?,
                euros_to_cents(purchase_price),
                notes,
            );
            let id = insert_property(&conn, &property)?;
            println!("Bien créé avec l'id {}", id);
        }

        Command::ListProperties => {
            println!("---------- List properties ----------"); // PROBLEM
            let properties = list_properties(&conn)?;
            if properties.is_empty() {
                println!("Aucun bien enregistré.");
            }
            for p in properties {
                println!(
                    "[{}] {} — {} — acheté le {} pour {:.2} €",
                    p.id.unwrap(),
                    p.label,
                    p.address,
                    p.purchase_date,
                    p.purchase_price_cents as f64 / 100.0
                );
            }
        }

        Command::AddTenant { name, contact } => {
            let tenant = Tenant::new(name, contact);
            let id = insert_tenant(&conn, &tenant)?;
            println!("Locataire créé avec l'id {}", id);
        }

        Command::AddLease {
            property_id,
            tenant_id,
            monthly_rent,
            start_date,
        } => {
            let lease = Lease::new(
                property_id,
                tenant_id,
                euros_to_cents(monthly_rent),
                parse_date(&start_date)?,
                None,
            );
            let id = insert_lease(&conn, &lease)?;
            println!("Bail créé avec l'id {}", id);
        }

        Command::AddExpense {
            property_id,
            category,
            amount,
            date,
            recurring,
        } => {
            let expense = Expense::new(
                property_id,
                category,
                euros_to_cents(amount),
                parse_date(&date)?,
                recurring,
            );
            let id = insert_expense(&conn, &expense)?;
            println!("Dépense enregistrée avec l'id {}", id);
        }

        Command::AddPayment {
            lease_id,
            amount,
            date,
            period,
        } => {
            let payment =
                RentPayment::new(lease_id, euros_to_cents(amount), parse_date(&date)?, period);
            let id = insert_rent_payment(&conn, &payment)?;
            println!("Paiement enregistré avec l'id {}", id);
        }

        Command::Profitability => {
            let results = all_properties_profitability(&conn)?;
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
        }

        Command::Overdue { up_to } => {
            let reference_date = match up_to {
                Some(s) => parse_date(&s)?,
                None => chrono::Local::now().date_naive(),
            };
            let overdue = all_overdue_leases(&conn, reference_date)?;
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
        }
    }
    Ok(())
}
