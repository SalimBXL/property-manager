use crate::error::{AppError, AppResult};
use chrono::NaiveDate;
use rusqlite::{Result as SqlResult, Row};

#[derive(Debug, Clone)]
pub struct Property {
    id: Option<i64>,
    label: String,
    address: String,
    purchase_date: NaiveDate,
    purchase_price_cents: i64,
    notes: Option<String>,
}

impl Property {
    pub fn new(
        label: String,
        address: String,
        purchase_date: NaiveDate,
        purchase_price_cents: i64,
        notes: Option<String>,
    ) -> AppResult<Self> {
        if purchase_price_cents < 0 {
            return Err(AppError::InvalidAmount(purchase_price_cents));
        }
        Ok(Property {
            id: None,
            label,
            address,
            purchase_date,
            purchase_price_cents,
            notes,
        })
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let id: i64 = row.get("id")?;
        let purchase_date_str: String = row.get("purchase_date")?;
        let purchase_date =
            NaiveDate::parse_from_str(&purchase_date_str, "%Y-%m-%d").map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    0,
                    format!("purchase_date='{}' (id={})", purchase_date_str, id),
                    rusqlite::types::Type::Text,
                )
            })?;
        Ok(Property {
            id: Some(id),
            label: row.get("label")?,
            address: row.get("address")?,
            purchase_date,
            purchase_price_cents: row.get("purchase_price_cents")?,
            notes: row.get("notes")?,
        })
    }

    pub fn id(&self) -> Option<i64> {
        self.id
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn purchase_date(&self) -> NaiveDate {
        self.purchase_date
    }

    pub fn purchase_price_cents(&self) -> i64 {
        self.purchase_price_cents
    }

    pub fn notes(&self) -> Option<&str> {
        self.notes.as_deref()
    }
}
