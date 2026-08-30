use crate::error::{AppError, AppResult};
use chrono::NaiveDate;
use rusqlite::{Result as SqlResult, Row};

#[derive(Debug, Clone)]
pub struct RentPayment {
    pub id: Option<i64>,
    pub lease_id: i64,
    pub amount_cents: i64,
    pub payment_date: NaiveDate,
    pub period_month: String, // format "YYYY-MM"
}

/// Vérifie que `period_month` respecte le format "YYYY-MM" avec un mois
/// valide (01-12). N'utilise pas `NaiveDate` : une période n'a pas de jour,
/// c'est un couple année/mois, pas une date complète.
fn validate_period_month(period_month: &str) -> AppResult<()> {
    let invalid = || AppError::InvalidPeriodMonth(period_month.to_string());

    let (year_str, month_str) = period_month.split_once('-').ok_or_else(invalid)?;

    if year_str.len() != 4 || month_str.len() != 2 {
        return Err(invalid());
    }
    if !year_str.bytes().all(|b| b.is_ascii_digit())
        || !month_str.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid());
    }

    let month: u32 = month_str.parse().map_err(|_| invalid())?;
    if !(1..=12).contains(&month) {
        return Err(invalid());
    }

    Ok(())
}

impl RentPayment {
    pub fn new(
        lease_id: i64,
        amount_cents: i64,
        payment_date: NaiveDate,
        period_month: String,
    ) -> AppResult<Self> {
        if amount_cents < 0 {
            return Err(AppError::InvalidAmount(amount_cents));
        }
        validate_period_month(&period_month)?;
        Ok(RentPayment {
            id: None,
            lease_id,
            amount_cents,
            payment_date,
            period_month,
        })
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let payment_date_str: String = row.get("payment_date")?;
        Ok(RentPayment {
            id: Some(row.get("id")?),
            lease_id: row.get("lease_id")?,
            amount_cents: row.get("amount_cents")?,
            payment_date: NaiveDate::parse_from_str(&payment_date_str, "%Y-%m-%d").map_err(
                |e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                },
            )?,
            period_month: row.get("period_month")?,
        })
    }
}
