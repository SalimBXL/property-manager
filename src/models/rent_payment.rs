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
