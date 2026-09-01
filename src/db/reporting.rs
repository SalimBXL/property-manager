use crate::db::repository::{
    active_lease_for_property, get_property, list_payments_for_lease, total_expenses_for_property,
};
use crate::error::AppError;
use crate::error::AppResult;
use crate::models::property::Property;
use chrono::{Datelike, NaiveDate};
use rusqlite::{Connection, params};
use std::collections::HashSet;

pub struct RentPaymentLine {
    pub tenant_name: String,
    pub period_month: String,
    pub payment_date: NaiveDate,
    pub amount_cents: i64,
}

pub fn list_rent_payments_for_property(
    conn: &Connection,
    property_id: i64,
) -> AppResult<Vec<RentPaymentLine>> {
    let mut stmt = conn.prepare(
        "SELECT t.name, rp.period_month, rp.payment_date, rp.amount_cents
         FROM rent_payment rp
         JOIN lease l ON rp.lease_id = l.id
         JOIN tenant t ON l.tenant_id = t.id
         WHERE l.property_id = ?1
         ORDER BY rp.period_month",
    )?;

    let raw_rows = stmt.query_map(params![property_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;

    let mut lines = Vec::new();
    for row in raw_rows {
        let (tenant_name, period_month, date_str, amount_cents) = row?;
        lines.push(RentPaymentLine {
            tenant_name,
            period_month,
            payment_date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?,
            amount_cents,
        });
    }
    Ok(lines)
}

// ---------- Rentabilité ----------

pub fn all_properties_profitability(conn: &Connection) -> AppResult<Vec<PropertyProfitability>> {
    let mut stmt = conn.prepare(
        "SELECT
            p.id,
            p.label,
            COALESCE(rent.total, 0) AS total_rent_collected,
            COALESCE(exp.total, 0) AS total_expenses
         FROM property p
         LEFT JOIN (
             SELECT l.property_id, SUM(rp.amount_cents) AS total
             FROM rent_payment rp
             JOIN lease l ON rp.lease_id = l.id
             GROUP BY l.property_id
         ) rent ON rent.property_id = p.id
         LEFT JOIN (
             SELECT property_id, SUM(amt) AS total FROM (
                 SELECT property_id, amount_cents AS amt
                 FROM expense WHERE expense_type = 'direct'
                 UNION ALL
                 SELECT property_id, amount_cents AS amt
                 FROM expense_allocation
             ) combined
             GROUP BY property_id
         ) exp ON exp.property_id = p.id
         ORDER BY p.label",
    )?;

    let rows = stmt.query_map([], |row| {
        let property_id: i64 = row.get(0)?;
        let label: String = row.get(1)?;
        let total_rent_collected: i64 = row.get(2)?;
        let total_expenses: i64 = row.get(3)?;
        Ok(PropertyProfitability {
            property_id,
            label,
            total_rent_collected,
            total_expenses,
            net_result: total_rent_collected - total_expenses,
        })
    })?;

    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub struct ExpenseLine {
    pub category: String,
    pub expense_date: NaiveDate,
    pub recurring: bool,
    pub is_indirect: bool,
    pub allocated_amount_cents: i64, // part de CE bien
    pub total_amount_cents: i64,     // montant total du frais (== allocated si direct)
}

pub fn list_expense_lines_for_property(
    conn: &Connection,
    property_id: i64,
) -> AppResult<Vec<ExpenseLine>> {
    let mut stmt = conn.prepare(
        "SELECT category, expense_date, recurring,
            amount_cents AS allocated, amount_cents AS total,
            0 AS is_indirect
     FROM expense
     WHERE property_id = ?1 AND expense_type = 'direct'
     UNION ALL
     SELECT e.category, e.expense_date, e.recurring,
            ea.amount_cents AS allocated, e.amount_cents AS total,
            1 AS is_indirect
     FROM expense_allocation ea
     JOIN expense e ON e.id = ea.expense_id
     WHERE ea.property_id = ?1
     ORDER BY expense_date",
    )?;

    let raw_rows = stmt.query_map(params![property_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;

    let mut lines = Vec::new();
    for row in raw_rows {
        let (category, date_str, recurring_int, allocated, total, is_indirect_int) = row?;
        lines.push(ExpenseLine {
            category,
            expense_date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?,
            recurring: recurring_int != 0,
            is_indirect: is_indirect_int != 0,
            allocated_amount_cents: allocated,
            total_amount_cents: total,
        });
    }
    Ok(lines)
}

pub struct PropertyProfitability {
    pub property_id: i64,
    pub label: String,
    pub total_rent_collected: i64,
    pub total_expenses: i64,
    pub net_result: i64,
}

pub fn property_profitability(
    conn: &Connection,
    property_id: i64,
) -> AppResult<PropertyProfitability> {
    // Réutilise get_property, qui renvoie déjà AppError::PropertyNotFound
    // proprement si le bien n'existe pas.
    let property = get_property(conn, property_id)?;

    let total_expenses = total_expenses_for_property(conn, property_id)?;

    let total_rent_collected: i64 = conn.query_row(
        "SELECT COALESCE(SUM(rp.amount_cents), 0)
         FROM rent_payment rp
         JOIN lease l ON rp.lease_id = l.id
         WHERE l.property_id = ?1",
        params![property_id],
        |row| row.get(0),
    )?;

    Ok(PropertyProfitability {
        property_id,
        label: property.label().to_string(),
        total_rent_collected,
        total_expenses,
        net_result: total_rent_collected - total_expenses,
    })
}

// ---------- Loyers en retard ----------

pub struct OverdueLease {
    pub lease_id: i64,
    pub property_label: String,
    pub tenant_name: String,
    pub missing_months: Vec<String>,
}

pub fn all_overdue_leases(conn: &Connection, up_to: NaiveDate) -> AppResult<Vec<OverdueLease>> {
    let mut stmt = conn.prepare(
        "SELECT l.id, p.label, t.name
         FROM lease l
         JOIN property p ON l.property_id = p.id
         JOIN tenant t ON l.tenant_id = t.id
         WHERE l.end_date IS NULL",
    )?;

    let active_leases: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut overdue = Vec::new();

    for (lease_id, property_label, tenant_name) in active_leases {
        let missing = missing_rent_months(conn, lease_id, up_to)?;
        if !missing.is_empty() {
            overdue.push(OverdueLease {
                lease_id,
                property_label,
                tenant_name,
                missing_months: missing,
            });
        }
    }

    Ok(overdue)
}

pub fn missing_rent_months(
    conn: &Connection,
    lease_id: i64,
    up_to: NaiveDate,
) -> AppResult<Vec<String>> {
    let start_date: String = conn.query_row(
        "SELECT start_date FROM lease WHERE id = ?1",
        params![lease_id],
        |row| row.get(0),
    )?;
    let start_date = NaiveDate::parse_from_str(&start_date, "%Y-%m-%d")?;

    let paid_months: HashSet<String> = list_payments_for_lease(conn, lease_id)?
        .into_iter()
        .map(|rp| rp.period_month)
        .collect();

    let mut missing = Vec::new();
    let mut year = start_date.year();
    let mut month = start_date.month();

    while (year, month) <= (up_to.year(), up_to.month()) {
        let period = format!("{:04}-{:02}", year, month);
        if !paid_months.contains(&period) {
            missing.push(period);
        }
        month += 1;
        if month > 12 {
            month = 1;
            year += 1;
        }
    }

    Ok(missing)
}

pub struct PropertyDetail {
    pub property: Property,
    pub expenses: Vec<ExpenseLine>,
    pub rent_payments: Vec<RentPaymentLine>,
    pub leases: Vec<LeaseHistoryLine>, // était Vec<Lease>
    pub total_rent_collected: i64,
    pub total_expenses: i64,
    pub net_result: i64,
    pub missing_months: Vec<String>,
}

pub fn property_detail(
    conn: &Connection,
    property_id: i64,
    up_to: NaiveDate,
) -> AppResult<PropertyDetail> {
    let property = get_property(conn, property_id)?;
    let expenses = list_expense_lines_for_property(conn, property_id)?;
    let rent_payments = list_rent_payments_for_property(conn, property_id)?;
    let leases = list_lease_history_for_property(conn, property_id)?; // changé
    let profitability = property_profitability(conn, property_id)?;

    let missing_months = match active_lease_for_property(conn, property_id)? {
        Some(lease) => {
            let lease_id = lease.id.ok_or_else(|| {
                AppError::Internal(
                    "un bail lu depuis la base doit toujours avoir un id".to_string(),
                )
            })?;
            missing_rent_months(conn, lease_id, up_to)?
        }
        None => Vec::new(),
    };

    Ok(PropertyDetail {
        property,
        expenses,
        rent_payments,
        leases,
        total_rent_collected: profitability.total_rent_collected,
        total_expenses: profitability.total_expenses,
        net_result: profitability.net_result,
        missing_months,
    })
}

pub struct LeaseHistoryLine {
    pub lease_id: i64,
    pub tenant_name: String,
    pub monthly_rent_cents: i64,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

impl LeaseHistoryLine {
    pub fn is_active(&self) -> bool {
        self.end_date.is_none()
    }
}

pub fn list_lease_history_for_property(
    conn: &Connection,
    property_id: i64,
) -> AppResult<Vec<LeaseHistoryLine>> {
    let mut stmt = conn.prepare(
        "SELECT l.id, t.name, l.monthly_rent_cents, l.start_date, l.end_date
         FROM lease l
         JOIN tenant t ON l.tenant_id = t.id
         WHERE l.property_id = ?1
         ORDER BY l.start_date",
    )?;

    let raw_rows = stmt.query_map(params![property_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<String>>(4)?,
        ))
    })?;

    let mut lines = Vec::new();
    for row in raw_rows {
        let (lease_id, tenant_name, monthly_rent_cents, start_str, end_str) = row?;
        let end_date = end_str
            .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
            .transpose()?;
        lines.push(LeaseHistoryLine {
            lease_id,
            tenant_name,
            monthly_rent_cents,
            start_date: NaiveDate::parse_from_str(&start_str, "%Y-%m-%d")?,
            end_date,
        });
    }
    Ok(lines)
}

pub struct ActiveLeaseSummary {
    pub lease_id: i64,
    pub property_label: String,
    pub tenant_name: String,
    pub monthly_rent_cents: i64,
    pub start_date: NaiveDate,
}

pub fn list_active_leases_with_names(conn: &Connection) -> AppResult<Vec<ActiveLeaseSummary>> {
    let mut stmt = conn.prepare(
        "SELECT l.id, p.label, t.name, l.monthly_rent_cents, l.start_date
         FROM lease l
         JOIN property p ON l.property_id = p.id
         JOIN tenant t ON l.tenant_id = t.id
         WHERE l.end_date IS NULL
         ORDER BY l.start_date",
    )?;

    let raw_rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let mut summaries = Vec::new();
    for row in raw_rows {
        let (lease_id, property_label, tenant_name, monthly_rent_cents, date_str) = row?;
        summaries.push(ActiveLeaseSummary {
            lease_id,
            property_label,
            tenant_name,
            monthly_rent_cents,
            start_date: NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?,
        });
    }
    Ok(summaries)
}

// ---------- Tests ----------

#[cfg(test)]
mod tests;
