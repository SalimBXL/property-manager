use chrono::NaiveDate;
use rusqlite::{Result as SqlResult, Row};

#[derive(Debug, Clone)]
pub struct Property {
    pub id: Option<i64>, // ← était `i64`
    pub label: String,
    pub address: String,
    pub purchase_date: NaiveDate,
    pub purchase_price_cents: i64,
    pub notes: Option<String>,
}

impl Property {
    pub fn new(
        label: String,
        address: String,
        purchase_date: NaiveDate,
        purchase_price_cents: i64,
        notes: Option<String>,
    ) -> Self {
        Property {
            id: None,
            label,
            address,
            purchase_date,
            purchase_price_cents,
            notes,
        }
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let purchase_date_str: String = row.get("purchase_date")?;
        Ok(Property {
            id: Some(row.get("id")?), // toujours Some, puisque ça vient de la base
            label: row.get("label")?,
            address: row.get("address")?,
            purchase_date: NaiveDate::parse_from_str(&purchase_date_str, "%Y-%m-%d").map_err(
                |e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                },
            )?,
            purchase_price_cents: row.get("purchase_price_cents")?,
            notes: row.get("notes")?,
        })
    }
}
