use chrono::NaiveDate;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("dates de bail invalides : fin ({end}) antérieure au début ({start})")]
    InvalidLeaseDates { start: NaiveDate, end: NaiveDate },

    #[error("le bien (id {0}) a déjà un bail actif")]
    PropertyAlreadyHasActiveLease(i64),

    #[error("le bien (id {0}) apparaît plusieurs fois dans la répartition du frais indirect")]
    DuplicatePropertyAllocation(i64),

    #[error("montant invalide : {0} centimes (doit être positif ou nul)")]
    InvalidAmount(i64),

    #[error("incohérence interne : {0}")]
    Internal(String),

    #[error("répartition impossible : aucun bien spécifié pour ce frais indirect")]
    EmptyAllocation,

    #[error("erreur base de données : {0}")]
    Database(#[from] rusqlite::Error),

    #[error("format de date invalide : {0}")]
    InvalidDate(#[from] chrono::ParseError),

    #[error("bien immobilier introuvable (id {0})")]
    PropertyNotFound(i64),

    #[error("bail introuvable (id {0})")]
    LeaseNotFound(i64),

    #[error(
        "impossible de supprimer le bien (id {0}) : des baux ou dépenses y sont encore rattachés"
    )]
    PropertyHasDependents(i64),

    #[error("erreur terminal : {0}")]
    Terminal(#[from] std::io::Error),
}

pub type AppResult<T> = Result<T, AppError>;
