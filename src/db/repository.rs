use rusqlite::{Connection, params};

use crate::error::{AppError, AppResult};
use crate::models::expense::Expense;
use crate::models::lease::Lease;
use crate::models::property::Property;
use crate::models::rent_payment::RentPayment;
use crate::models::tenant::Tenant;
use chrono::NaiveDate;

// ---------- Property ----------

pub fn insert_property(conn: &Connection, p: &Property) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO property (label, address, purchase_date, purchase_price_cents, notes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            p.label,
            p.address,
            p.purchase_date.format("%Y-%m-%d").to_string(),
            p.purchase_price_cents,
            p.notes,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_property(conn: &Connection, id: i64) -> AppResult<Property> {
    conn.query_row(
        "SELECT id, label, address, purchase_date, purchase_price_cents, notes
         FROM property WHERE id = ?1",
        params![id],
        Property::from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::PropertyNotFound(id),
        other => AppError::Database(other),
    })
}

pub fn list_properties(conn: &Connection) -> AppResult<Vec<Property>> {
    let mut stmt = conn.prepare(
        "SELECT id, label, address, purchase_date, purchase_price_cents, notes
         FROM property ORDER BY purchase_date",
    )?;
    let rows = stmt.query_map([], Property::from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn delete_property(conn: &Connection, id: i64) -> AppResult<()> {
    conn.execute("DELETE FROM property WHERE id = ?1", params![id])
        .map_err(|e| match &e {
            rusqlite::Error::SqliteFailure(err, _)
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                AppError::PropertyHasDependents(id)
            }
            _ => AppError::Database(e),
        })?;
    Ok(())
}

// ---------- Expense ----------

/// Répartit un montant en centimes en `n` parts aussi égales que possible.
/// Le reste (dû à la division entière) est distribué aux premières parts,
/// pour garantir que la somme des parts == montant total au centime près.
fn split_evenly(total_cents: i64, n: usize) -> Vec<i64> {
    let base = total_cents / n as i64;
    let remainder = total_cents % n as i64;
    (0..n as i64)
        .map(|i| if i < remainder { base + 1 } else { base })
        .collect()
}

pub fn insert_indirect_expense(
    conn: &Connection,
    category: &str,
    total_amount_cents: i64,
    expense_date: NaiveDate,
    recurring: bool,
    property_ids: &[i64],
) -> AppResult<i64> {
    if property_ids.is_empty() {
        return Err(AppError::EmptyAllocation);
    }

    let shares = split_evenly(total_amount_cents, property_ids.len());

    // unchecked_transaction : disponible sur &Connection (pas besoin de &mut),
    // cohérent avec le reste du repository qui prend &Connection partout.
    let tx = conn.unchecked_transaction()?;

    tx.execute(
        "INSERT INTO expense (property_id, category, amount_cents, expense_date, recurring, expense_type)
         VALUES (NULL, ?1, ?2, ?3, ?4, 'indirect')",
        params![
            category,
            total_amount_cents,
            expense_date.format("%Y-%m-%d").to_string(),
            recurring as i64,
        ],
    )?;
    let expense_id = tx.last_insert_rowid();

    for (property_id, share) in property_ids.iter().zip(shares.iter()) {
        tx.execute(
            "INSERT INTO expense_allocation (expense_id, property_id, amount_cents)
             VALUES (?1, ?2, ?3)",
            params![expense_id, property_id, share],
        )?;
    }

    tx.commit()?;
    Ok(expense_id)
}

pub fn insert_expense(conn: &Connection, e: &Expense) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO expense (property_id, category, amount_cents, expense_date, recurring, expense_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            e.property_id,
            e.category,
            e.amount_cents,
            e.expense_date.format("%Y-%m-%d").to_string(),
            e.recurring as i64,
            e.expense_type.as_str(),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_expenses_for_property(conn: &Connection, property_id: i64) -> AppResult<Vec<Expense>> {
    let mut stmt = conn.prepare(
        "SELECT id, property_id, category, amount_cents, expense_date, recurring
         FROM expense WHERE property_id = ?1 ORDER BY expense_date",
    )?;
    let rows = stmt.query_map(params![property_id], Expense::from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn total_expenses_for_property(conn: &Connection, property_id: i64) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT
            COALESCE((SELECT SUM(amount_cents) FROM expense
                      WHERE property_id = ?1 AND expense_type = 'direct'), 0)
            +
            COALESCE((SELECT SUM(amount_cents) FROM expense_allocation
                      WHERE property_id = ?1), 0)",
        params![property_id],
        |row| row.get(0),
    )?)
}

// ---------- Tenant ----------

pub fn insert_tenant(conn: &Connection, t: &Tenant) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO tenant (name, contact) VALUES (?1, ?2)",
        params![t.name, t.contact],
    )?;
    Ok(conn.last_insert_rowid())
}

// ---------- Lease ----------

// src/db/repository.rs — section Lease

pub fn list_leases_for_property(conn: &Connection, property_id: i64) -> AppResult<Vec<Lease>> {
    let mut stmt = conn.prepare(
        "SELECT id, property_id, tenant_id, monthly_rent_cents, start_date, end_date
         FROM lease WHERE property_id = ?1 ORDER BY start_date",
    )?;
    let rows = stmt.query_map(params![property_id], Lease::from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn insert_lease(conn: &Connection, l: &Lease) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO lease (property_id, tenant_id, monthly_rent_cents, start_date, end_date)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            l.property_id,
            l.tenant_id,
            l.monthly_rent_cents,
            l.start_date.format("%Y-%m-%d").to_string(),
            l.end_date.map(|d| d.format("%Y-%m-%d").to_string()),
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Retourne le bail actif d'un bien, si il existe. `None` est un résultat
/// normal ici (pas d'erreur) — un bien peut légitimement n'avoir aucun
/// bail actif.
pub fn active_lease_for_property(conn: &Connection, property_id: i64) -> AppResult<Option<Lease>> {
    let result = conn.query_row(
        "SELECT id, property_id, tenant_id, monthly_rent_cents, start_date, end_date
         FROM lease WHERE property_id = ?1 AND end_date IS NULL",
        params![property_id],
        Lease::from_row,
    );

    match result {
        Ok(lease) => Ok(Some(lease)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(other) => Err(AppError::Database(other)),
    }
}

/// Récupère un bail par id, erreur explicite s'il n'existe pas.
pub fn get_lease(conn: &Connection, id: i64) -> AppResult<Lease> {
    conn.query_row(
        "SELECT id, property_id, tenant_id, monthly_rent_cents, start_date, end_date
         FROM lease WHERE id = ?1",
        params![id],
        Lease::from_row,
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::LeaseNotFound(id),
        other => AppError::Database(other),
    })
}

/// Liste tous les baux actifs (end_date IS NULL), tous biens confondus.
pub fn list_active_leases(conn: &Connection) -> AppResult<Vec<Lease>> {
    let mut stmt = conn.prepare(
        "SELECT id, property_id, tenant_id, monthly_rent_cents, start_date, end_date
         FROM lease WHERE end_date IS NULL
         ORDER BY start_date",
    )?;
    let rows = stmt.query_map([], Lease::from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

// ---------- RentPayment ----------

pub fn insert_rent_payment(conn: &Connection, rp: &RentPayment) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO rent_payment (lease_id, amount_cents, payment_date, period_month)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            rp.lease_id,
            rp.amount_cents,
            rp.payment_date.format("%Y-%m-%d").to_string(),
            rp.period_month,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn list_payments_for_lease(conn: &Connection, lease_id: i64) -> AppResult<Vec<RentPayment>> {
    let mut stmt = conn.prepare(
        "SELECT id, lease_id, amount_cents, payment_date, period_month
         FROM rent_payment WHERE lease_id = ?1 ORDER BY period_month",
    )?;
    let rows = stmt.query_map(params![lease_id], RentPayment::from_row)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

pub fn total_paid_for_lease(conn: &Connection, lease_id: i64) -> AppResult<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(amount_cents), 0) FROM rent_payment WHERE lease_id = ?1",
        params![lease_id],
        |row| row.get(0),
    )?)
}

// ---------- Tests ----------

#[cfg(test)]
mod tests;
