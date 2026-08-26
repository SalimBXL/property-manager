use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
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

    #[error("date invalide en base pour le bien (id {id}) : '{value}'")]
    CorruptedDateData { id: i64, value: String },
}

pub type AppResult<T> = Result<T, AppError>;
