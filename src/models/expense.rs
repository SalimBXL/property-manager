use chrono::NaiveDate;
use rusqlite::{Result as SqlResult, Row};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpenseType {
    Direct,
    Indirect,
}

impl ExpenseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExpenseType::Direct => "direct",
            ExpenseType::Indirect => "indirect",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "indirect" => ExpenseType::Indirect,
            _ => ExpenseType::Direct,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expense {
    pub id: Option<i64>,
    pub property_id: Option<i64>, // None si frais indirect
    pub category: String,
    pub amount_cents: i64, // montant total
    pub expense_date: NaiveDate,
    pub recurring: bool,
    pub expense_type: ExpenseType,
}

impl Expense {
    /// Frais direct : rattaché à un seul bien.
    pub fn new(
        property_id: i64,
        category: String,
        amount_cents: i64,
        expense_date: NaiveDate,
        recurring: bool,
    ) -> Self {
        Expense {
            id: None,
            property_id: Some(property_id),
            category,
            amount_cents,
            expense_date,
            recurring,
            expense_type: ExpenseType::Direct,
        }
    }

    /// Frais indirect : pas encore rattaché à un bien précis, sera réparti
    /// via `expense_allocation` au moment de l'insertion en base.
    pub fn new_indirect(
        category: String,
        amount_cents: i64,
        expense_date: NaiveDate,
        recurring: bool,
    ) -> Self {
        Expense {
            id: None,
            property_id: None,
            category,
            amount_cents,
            expense_date,
            recurring,
            expense_type: ExpenseType::Indirect,
        }
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let expense_date_str: String = row.get("expense_date")?;
        let recurring_int: i64 = row.get("recurring")?;
        let type_str: String = row.get("expense_type")?;
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
            expense_type: ExpenseType::from_str(&type_str),
        })
    }
}
