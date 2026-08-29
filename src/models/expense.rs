use chrono::NaiveDate;
use rusqlite::{Result as SqlResult, Row};
use thiserror::Error;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpenseTarget {
    Direct { property_id: i64 },
    Indirect,
}

impl ExpenseTarget {
    pub fn is_indirect(&self) -> bool {
        matches!(self, ExpenseTarget::Indirect)
    }

    pub fn property_id(&self) -> Option<i64> {
        match self {
            ExpenseTarget::Direct { property_id } => Some(*property_id),
            ExpenseTarget::Indirect => None,
        }
    }

    pub fn type_str(&self) -> &'static str {
        match self {
            ExpenseTarget::Direct { .. } => "direct",
            ExpenseTarget::Indirect => "indirect",
        }
    }

    /// Reconstruit une cible à partir des deux colonnes SQLite (`property_id`
    /// nullable + `expense_type` texte), en rejetant les combinaisons
    /// incohérentes plutôt que de choisir une valeur par défaut arbitraire.
    fn from_storage(property_id: Option<i64>, type_str: &str) -> Result<Self, ExpenseTargetError> {
        match (type_str, property_id) {
            ("direct", Some(id)) => Ok(ExpenseTarget::Direct { property_id: id }),
            ("indirect", None) => Ok(ExpenseTarget::Indirect),
            ("direct", None) => Err(ExpenseTargetError::DirectWithoutProperty),
            ("indirect", Some(id)) => Err(ExpenseTargetError::IndirectWithProperty(id)),
            (other, _) => Err(ExpenseTargetError::InvalidType(other.to_string())),
        }
    }
}

#[derive(Debug, Error)]
pub enum ExpenseTargetError {
    #[error("type de dépense invalide : '{0}'")]
    InvalidType(String),
    #[error("frais direct sans bien associé (property_id manquant)")]
    DirectWithoutProperty,
    #[error("frais indirect associé à un bien (id {0}), ce qui est incohérent")]
    IndirectWithProperty(i64),
}

#[derive(Debug, Clone)]
pub struct Expense {
    id: Option<i64>,
    target: ExpenseTarget,
    category: String,
    amount_cents: i64,
    expense_date: NaiveDate,
    recurring: bool,
}

impl Expense {
    pub fn new_direct(
        property_id: i64,
        category: String,
        amount_cents: i64,
        expense_date: NaiveDate,
        recurring: bool,
    ) -> AppResult<Self> {
        if amount_cents < 0 {
            return Err(AppError::InvalidAmount(amount_cents));
        }
        Ok(Expense {
            id: None,
            target: ExpenseTarget::Direct { property_id },
            category,
            amount_cents,
            expense_date,
            recurring,
        })
    }

    pub fn new_indirect(
        category: String,
        amount_cents: i64,
        expense_date: NaiveDate,
        recurring: bool,
    ) -> AppResult<Self> {
        if amount_cents < 0 {
            return Err(AppError::InvalidAmount(amount_cents));
        }
        Ok(Expense {
            id: None,
            target: ExpenseTarget::Indirect,
            category,
            amount_cents,
            expense_date,
            recurring,
        })
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let expense_date_str: String = row.get("expense_date")?;
        let recurring_int: i64 = row.get("recurring")?;
        let type_str: String = row.get("expense_type")?;
        let property_id: Option<i64> = row.get("property_id")?;

        let target = ExpenseTarget::from_storage(property_id, &type_str).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;

        Ok(Expense {
            id: Some(row.get("id")?),
            target,
            category: row.get("category")?,
            amount_cents: row.get("amount_cents")?,
            expense_date: NaiveDate::parse_from_str(&expense_date_str, "%Y-%m-%d").map_err(
                |e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                },
            )?,
            recurring: recurring_int != 0,
        })
    }

    pub fn id(&self) -> Option<i64> {
        self.id
    }

    pub fn target(&self) -> ExpenseTarget {
        self.target
    }

    pub fn is_indirect(&self) -> bool {
        self.target.is_indirect()
    }

    pub fn property_id(&self) -> Option<i64> {
        self.target.property_id()
    }

    pub fn category(&self) -> &str {
        &self.category
    }

    pub fn amount_cents(&self) -> i64 {
        self.amount_cents
    }

    pub fn expense_date(&self) -> NaiveDate {
        self.expense_date
    }

    pub fn recurring(&self) -> bool {
        self.recurring
    }
}
