use chrono::NaiveDate;
use rusqlite::{Result as SqlResult, Row};

#[derive(Debug, Clone)]
pub struct Expense {
    pub id: Option<i64>,
    pub property_id: i64,
    pub category: String,
    pub amount_cents: i64,
    pub expense_date: NaiveDate,
    pub recurring: bool,
}

impl Expense {
    pub fn new(
        property_id: i64,
        category: String,
        amount_cents: i64,
        expense_date: NaiveDate,
        recurring: bool,
    ) -> Self {
        Expense {
            id: None,
            property_id,
            category,
            amount_cents,
            expense_date,
            recurring,
        }
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let expense_date_str: String = row.get("expense_date")?;
        let recurring_int: i64 = row.get("recurring")?;
        Ok(Expense {
            id: Some(row.get("id")?),
            property_id: row.get("property_id")?,
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
}
