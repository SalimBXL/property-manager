use crate::error::{AppError, AppResult};
use chrono::NaiveDate;
use rusqlite::{Result as SqlResult, Row};

#[derive(Debug, Clone)]
pub struct Lease {
    pub id: Option<i64>,
    pub property_id: i64,
    pub tenant_id: i64,
    pub monthly_rent_cents: i64,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

impl Lease {
    pub fn new(
        property_id: i64,
        tenant_id: i64,
        monthly_rent_cents: i64,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
    ) -> AppResult<Self> {
        if monthly_rent_cents < 0 {
            return Err(AppError::InvalidAmount(monthly_rent_cents));
        }
        if let Some(end) = end_date
            && end <= start_date
        {
            return Err(AppError::InvalidLeaseDates {
                start: start_date,
                end,
            });
        }
        Ok(Lease {
            id: None,
            property_id,
            tenant_id,
            monthly_rent_cents,
            start_date,
            end_date,
        })
    }

    fn parse_date(s: &str) -> SqlResult<NaiveDate> {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        let start_date_str: String = row.get("start_date")?;
        let end_date_str: Option<String> = row.get("end_date")?;

        let end_date = end_date_str.map(|s| Self::parse_date(&s)).transpose()?;

        Ok(Lease {
            id: Some(row.get("id")?),
            property_id: row.get("property_id")?,
            tenant_id: row.get("tenant_id")?,
            monthly_rent_cents: row.get("monthly_rent_cents")?,
            start_date: Self::parse_date(&start_date_str)?,
            end_date,
        })
    }

    pub fn is_active(&self) -> bool {
        self.end_date.is_none()
    }
}
