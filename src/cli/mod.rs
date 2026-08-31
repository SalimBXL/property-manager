mod dispatch;
mod handlers;
mod inputs;

pub use dispatch::run_command;

use chrono::NaiveDate;
use clap::{Parser, Subcommand};

use property_manager::error::AppResult;

#[derive(Parser)]
#[command(name = "property-manager", about = "Gestion de biens immobiliers")]
pub struct Cli {
    /// Chemin vers le fichier de base de données SQLite
    #[arg(long, default_value = "property_manager.db")]
    pub db_path: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
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

    /// Modifier un bien existant
    UpdateProperty {
        property_id: i64,
        label: String,
        address: String,
        purchase_date: String,
        purchase_price: f64,
        #[arg(long)]
        notes: Option<String>,
    },

    /// Modifier un locataire existant
    UpdateTenant {
        tenant_id: i64,
        name: String,
        #[arg(long)]
        contact: Option<String>,
    },

    /// Modifier une dépense directe existante
    UpdateExpense {
        expense_id: i64,
        property_id: i64,
        category: String,
        amount: f64,
        date: String,
        #[arg(long)]
        recurring: bool,
    },
}

fn parse_date(s: &str) -> AppResult<NaiveDate> {
    Ok(NaiveDate::parse_from_str(s, "%Y-%m-%d")?)
}

/// Convertit un montant en euros (saisi par l'utilisateur) en centimes.
fn euros_to_cents(euros: f64) -> i64 {
    (euros * 100.0).round() as i64
}
