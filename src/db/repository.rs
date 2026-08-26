use rusqlite::{Connection, params};

use crate::error::{AppError, AppResult};
use crate::models::expense::Expense;
use crate::models::lease::Lease;
use crate::models::property::Property;
use crate::models::rent_payment::RentPayment;
use crate::models::tenant::Tenant;

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

pub fn insert_expense(conn: &Connection, e: &Expense) -> AppResult<i64> {
    conn.execute(
        "INSERT INTO expense (property_id, category, amount_cents, expense_date, recurring)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            e.property_id,
            e.category,
            e.amount_cents,
            e.expense_date.format("%Y-%m-%d").to_string(),
            e.recurring as i64,
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
        "SELECT COALESCE(SUM(amount_cents), 0) FROM expense WHERE property_id = ?1",
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
mod tests {
    use super::*;
    use crate::db;
    use chrono::NaiveDate;

    #[test]
    fn test_insert_and_get_property() {
        let conn = db::open_in_memory().unwrap();
        let p = Property::new(
            "Parking A12".to_string(),
            "Rue de la Gare 10".to_string(),
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            1_500_000,
            None,
        );

        let id = insert_property(&conn, &p).unwrap();
        let fetched = get_property(&conn, id).unwrap();

        assert_eq!(fetched.label, "Parking A12");
        assert_eq!(fetched.purchase_price_cents, 1_500_000);
    }

    #[test]
    fn test_expense_total() {
        let conn = db::open_in_memory().unwrap();
        let p = Property::new(
            "Parking A12".to_string(),
            "Rue de la Gare 10".to_string(),
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            1_500_000,
            None,
        );
        let property_id = insert_property(&conn, &p).unwrap();

        let e1 = Expense::new(
            property_id,
            "taxe foncière".to_string(),
            25_000,
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            true,
        );
        let e2 = Expense::new(
            property_id,
            "réparation".to_string(),
            8_000,
            NaiveDate::from_ymd_opt(2024, 9, 12).unwrap(),
            false,
        );

        insert_expense(&conn, &e1).unwrap();
        insert_expense(&conn, &e2).unwrap();

        let total = total_expenses_for_property(&conn, property_id).unwrap();
        assert_eq!(total, 33_000);
    }

    #[test]
    fn test_insert_lease_with_tenant() {
        let conn = db::open_in_memory().unwrap();

        let p = Property::new(
            "Parking A12".to_string(),
            "Rue de la Gare 10".to_string(),
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            1_500_000,
            None,
        );
        let property_id = insert_property(&conn, &p).unwrap();

        let t = Tenant::new(
            "Jean Dupont".to_string(),
            Some("jean@example.com".to_string()),
        );
        let tenant_id = insert_tenant(&conn, &t).unwrap();

        let l = Lease::new(
            property_id,
            tenant_id,
            8_000,
            NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
            None,
        );
        insert_lease(&conn, &l).unwrap();

        let active = active_lease_for_property(&conn, property_id).unwrap();
        assert!(active.is_some());

        let active_lease = active.unwrap();
        assert_eq!(active_lease.tenant_id, tenant_id);
        assert!(active_lease.is_active());
    }

    #[test]
    fn test_no_active_lease_when_ended() {
        let conn = db::open_in_memory().unwrap();

        let p = Property::new(
            "Parking B3".to_string(),
            "Rue du Marché 5".to_string(),
            NaiveDate::from_ymd_opt(2023, 1, 10).unwrap(),
            1_200_000,
            None,
        );
        let property_id = insert_property(&conn, &p).unwrap();

        let t = Tenant::new("Marie Leroy".to_string(), None);
        let tenant_id = insert_tenant(&conn, &t).unwrap();

        let l = Lease::new(
            property_id,
            tenant_id,
            7_500,
            NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            Some(NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()),
        );
        insert_lease(&conn, &l).unwrap();

        let active = active_lease_for_property(&conn, property_id).unwrap();
        assert!(active.is_none());
    }

    #[test]
    fn test_rent_payment_total() {
        let conn = db::open_in_memory().unwrap();

        let p = Property::new(
            "Parking A12".to_string(),
            "Rue de la Gare 10".to_string(),
            NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
            1_500_000,
            None,
        );
        let property_id = insert_property(&conn, &p).unwrap();

        let t = Tenant::new("Jean Dupont".to_string(), None);
        let tenant_id = insert_tenant(&conn, &t).unwrap();

        let l = Lease::new(
            property_id,
            tenant_id,
            8_000,
            NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
            None,
        );
        let lease_id = insert_lease(&conn, &l).unwrap();

        let rp1 = RentPayment::new(
            lease_id,
            8_000,
            NaiveDate::from_ymd_opt(2024, 4, 3).unwrap(),
            "2024-04".to_string(),
        );
        let rp2 = RentPayment::new(
            lease_id,
            8_000,
            NaiveDate::from_ymd_opt(2024, 5, 2).unwrap(),
            "2024-05".to_string(),
        );

        insert_rent_payment(&conn, &rp1).unwrap();
        insert_rent_payment(&conn, &rp2).unwrap();

        let total = total_paid_for_lease(&conn, lease_id).unwrap();
        assert_eq!(total, 16_000);
    }

    #[test]
    fn deleting_property_with_dependents_is_blocked() {
        let conn = db::open_in_memory().unwrap();

        let property = Property::new(
            "Parking D1".to_string(),
            "Rue Neuve 1".to_string(),
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            900_000,
            None,
        );
        let property_id = insert_property(&conn, &property).unwrap();

        let tenant = Tenant::new("Marc Petit".to_string(), None);
        let tenant_id = insert_tenant(&conn, &tenant).unwrap();

        let lease = Lease::new(
            property_id,
            tenant_id,
            6_000,
            NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
            None,
        );
        insert_lease(&conn, &lease).unwrap();

        // La suppression doit être refusée tant qu'un bail y est rattaché
        let result = delete_property(&conn, property_id);
        assert!(matches!(result, Err(AppError::PropertyHasDependents(id)) if id == property_id));

        // Le bien existe toujours
        let still_there = get_property(&conn, property_id);
        assert!(still_there.is_ok());
    }

    #[test]
    fn deleting_property_without_dependents_succeeds() {
        let conn = db::open_in_memory().unwrap();

        let property = Property::new(
            "Parking D2".to_string(),
            "Rue Neuve 2".to_string(),
            NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
            900_000,
            None,
        );
        let property_id = insert_property(&conn, &property).unwrap();

        delete_property(&conn, property_id).unwrap();

        let result = get_property(&conn, property_id);
        assert!(result.is_err());
    }
}
