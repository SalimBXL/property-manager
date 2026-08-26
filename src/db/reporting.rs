use chrono::{Datelike, NaiveDate};
use rusqlite::{Connection, params};
use std::collections::HashSet;

use crate::db::repository::{get_property, list_payments_for_lease, total_expenses_for_property};
use crate::error::AppResult;

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
             SELECT property_id, SUM(amount_cents) AS total
             FROM expense
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
        label: property.label,
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

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::db::repository::{
        insert_expense, insert_lease, insert_property, insert_rent_payment, insert_tenant,
    };
    use crate::models::expense::Expense;
    use crate::models::lease::Lease;
    use crate::models::property::Property;
    use crate::models::rent_payment::RentPayment;
    use crate::models::tenant::Tenant;

    #[test]
    fn test_property_profitability() {
        let conn = db::open_in_memory().unwrap();

        let p = Property::new(
            "Parking A12".to_string(),
            "Rue de la Gare 10".to_string(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
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
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
        );
        let lease_id = insert_lease(&conn, &l).unwrap();

        insert_rent_payment(
            &conn,
            &RentPayment::new(
                lease_id,
                8_000,
                NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                "2024-01".to_string(),
            ),
        )
        .unwrap();
        insert_rent_payment(
            &conn,
            &RentPayment::new(
                lease_id,
                8_000,
                NaiveDate::from_ymd_opt(2024, 2, 3).unwrap(),
                "2024-02".to_string(),
            ),
        )
        .unwrap();

        insert_expense(
            &conn,
            &Expense::new(
                property_id,
                "taxe".to_string(),
                5_000,
                NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
                true,
            ),
        )
        .unwrap();

        let result = property_profitability(&conn, property_id).unwrap();
        assert_eq!(result.total_rent_collected, 16_000);
        assert_eq!(result.total_expenses, 5_000);
        assert_eq!(result.net_result, 11_000);
    }

    #[test]
    fn test_missing_rent_months_crosses_year() {
        let conn = db::open_in_memory().unwrap();

        let p = Property::new(
            "Parking B3".to_string(),
            "Rue du Marché 5".to_string(),
            NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
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
            NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
            None,
        );
        let lease_id = insert_lease(&conn, &l).unwrap();

        insert_rent_payment(
            &conn,
            &RentPayment::new(
                lease_id,
                7_500,
                NaiveDate::from_ymd_opt(2023, 11, 5).unwrap(),
                "2023-11".to_string(),
            ),
        )
        .unwrap();
        insert_rent_payment(
            &conn,
            &RentPayment::new(
                lease_id,
                7_500,
                NaiveDate::from_ymd_opt(2024, 1, 4).unwrap(),
                "2024-01".to_string(),
            ),
        )
        .unwrap();

        let up_to = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let missing = missing_rent_months(&conn, lease_id, up_to).unwrap();

        assert_eq!(missing, vec!["2023-12".to_string()]);
    }

    #[test]
    fn test_all_properties_profitability() {
        let conn = db::open_in_memory().unwrap();

        // Bien 1 : loyers + dépenses
        let p1 = Property::new(
            "Parking A12".to_string(),
            "Rue de la Gare 10".to_string(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            1_500_000,
            None,
        );
        let p1_id = insert_property(&conn, &p1).unwrap();
        let t1 = Tenant::new("Jean Dupont".to_string(), None);
        let t1_id = insert_tenant(&conn, &t1).unwrap();
        let l1 = Lease::new(
            p1_id,
            t1_id,
            8_000,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
        );
        let l1_id = insert_lease(&conn, &l1).unwrap();
        insert_rent_payment(
            &conn,
            &RentPayment::new(
                l1_id,
                8_000,
                NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                "2024-01".to_string(),
            ),
        )
        .unwrap();
        insert_expense(
            &conn,
            &Expense::new(
                p1_id,
                "taxe".to_string(),
                3_000,
                NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
                true,
            ),
        )
        .unwrap();

        // Bien 2 : ni loyer ni dépense, doit quand même apparaître avec des zéros
        let p2 = Property::new(
            "Parking B3".to_string(),
            "Rue du Marché 5".to_string(),
            NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            1_000_000,
            None,
        );
        insert_property(&conn, &p2).unwrap();

        let results = all_properties_profitability(&conn).unwrap();

        assert_eq!(results.len(), 2);

        let r1 = results.iter().find(|r| r.property_id == p1_id).unwrap();
        assert_eq!(r1.total_rent_collected, 8_000);
        assert_eq!(r1.total_expenses, 3_000);
        assert_eq!(r1.net_result, 5_000);

        let r2 = results.iter().find(|r| r.label == "Parking B3").unwrap();
        assert_eq!(r2.total_rent_collected, 0);
        assert_eq!(r2.total_expenses, 0);
        assert_eq!(r2.net_result, 0);
    }

    #[test]
    fn test_all_overdue_leases() {
        let conn = db::open_in_memory().unwrap();

        // Bail à jour
        let p1 = Property::new(
            "Parking A12".to_string(),
            "Rue de la Gare 10".to_string(),
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            1_500_000,
            None,
        );
        let p1_id = insert_property(&conn, &p1).unwrap();
        let t1 = Tenant::new("Jean Dupont".to_string(), None);
        let t1_id = insert_tenant(&conn, &t1).unwrap();
        let l1 = Lease::new(
            p1_id,
            t1_id,
            8_000,
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            None,
        );
        let l1_id = insert_lease(&conn, &l1).unwrap();
        insert_rent_payment(
            &conn,
            &RentPayment::new(
                l1_id,
                8_000,
                NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
                "2024-01".to_string(),
            ),
        )
        .unwrap();

        // Bail avec un mois manquant
        let p2 = Property::new(
            "Parking B3".to_string(),
            "Rue du Marché 5".to_string(),
            NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
            1_200_000,
            None,
        );
        let p2_id = insert_property(&conn, &p2).unwrap();
        let t2 = Tenant::new("Marie Leroy".to_string(), None);
        let t2_id = insert_tenant(&conn, &t2).unwrap();
        let l2 = Lease::new(
            p2_id,
            t2_id,
            7_500,
            NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
            None,
        );
        let l2_id = insert_lease(&conn, &l2).unwrap();
        insert_rent_payment(
            &conn,
            &RentPayment::new(
                l2_id,
                7_500,
                NaiveDate::from_ymd_opt(2023, 11, 5).unwrap(),
                "2023-11".to_string(),
            ),
        )
        .unwrap();
        // décembre non payé

        let up_to = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        let overdue = all_overdue_leases(&conn, up_to).unwrap();

        assert_eq!(overdue.len(), 1);
        assert_eq!(overdue[0].lease_id, l2_id);
        assert_eq!(overdue[0].tenant_name, "Marie Leroy");
        assert_eq!(
            overdue[0].missing_months,
            vec!["2023-12".to_string(), "2024-01".to_string()]
        );
    }
}
