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
use rusqlite::Connection;

/// Crée un bien de test et retourne son id, avec `.unwrap()` déjà géré.
fn make_property(
    conn: &Connection,
    label: &str,
    address: &str,
    purchase_date: NaiveDate,
    price_cents: i64,
) -> i64 {
    let p = Property::new(
        label.to_string(),
        address.to_string(),
        purchase_date,
        price_cents,
        None,
    )
    .unwrap();
    insert_property(conn, &p).unwrap()
}

/// Crée un locataire de test et retourne son id.
fn make_tenant(conn: &Connection, name: &str) -> i64 {
    insert_tenant(conn, &Tenant::new(name.to_string(), None)).unwrap()
}

/// Crée un bail actif de test et retourne son id.
fn make_lease(
    conn: &Connection,
    property_id: i64,
    tenant_id: i64,
    monthly_rent_cents: i64,
    start_date: NaiveDate,
) -> i64 {
    let l = Lease::new(property_id, tenant_id, monthly_rent_cents, start_date, None).unwrap();
    insert_lease(conn, &l).unwrap()
}

/// Enregistre un paiement de loyer de test.
fn make_payment(
    conn: &Connection,
    lease_id: i64,
    amount_cents: i64,
    date: NaiveDate,
    period: &str,
) {
    let rp = RentPayment::new(lease_id, amount_cents, date, period.to_string()).unwrap();
    insert_rent_payment(conn, &rp).unwrap();
}

/// Enregistre une dépense directe de test.
fn make_expense(
    conn: &Connection,
    property_id: i64,
    category: &str,
    amount_cents: i64,
    date: NaiveDate,
) {
    let e =
        Expense::new_direct(property_id, category.to_string(), amount_cents, date, true).unwrap();
    insert_expense(conn, &e).unwrap();
}

#[test]
fn test_property_profitability() {
    let conn = db::open_in_memory().unwrap();
    let property_id = make_property(
        &conn,
        "Parking A12",
        "Rue de la Gare 10",
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_500_000,
    );
    let tenant_id = make_tenant(&conn, "Jean Dupont");
    let lease_id = make_lease(
        &conn,
        property_id,
        tenant_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    );

    make_payment(
        &conn,
        lease_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        "2024-01",
    );
    make_payment(
        &conn,
        lease_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 2, 3).unwrap(),
        "2024-02",
    );
    make_expense(
        &conn,
        property_id,
        "taxe",
        5_000,
        NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
    );

    let result = property_profitability(&conn, property_id).unwrap();
    assert_eq!(result.total_rent_collected, 16_000);
    assert_eq!(result.total_expenses, 5_000);
    assert_eq!(result.net_result, 11_000);
}

#[test]
fn test_missing_rent_months_crosses_year() {
    let conn = db::open_in_memory().unwrap();

    let property_id = make_property(
        &conn,
        "Parking B3",
        "Rue du Marché 5",
        NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
        1_200_000,
    );
    let tenant_id = make_tenant(&conn, "Marie Leroy");
    let lease_id = make_lease(
        &conn,
        property_id,
        tenant_id,
        7_500,
        NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
    );

    make_payment(
        &conn,
        lease_id,
        7_500,
        NaiveDate::from_ymd_opt(2023, 11, 5).unwrap(),
        "2023-11",
    );
    make_payment(
        &conn,
        lease_id,
        7_500,
        NaiveDate::from_ymd_opt(2024, 1, 4).unwrap(),
        "2024-01",
    );

    let up_to = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
    let missing = missing_rent_months(&conn, lease_id, up_to).unwrap();

    assert_eq!(missing, vec!["2023-12".to_string()]);
}

#[test]
fn test_all_properties_profitability() {
    let conn = db::open_in_memory().unwrap();

    let p1_id = make_property(
        &conn,
        "Parking A12",
        "Rue de la Gare 10",
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_500_000,
    );
    let t1_id = make_tenant(&conn, "Jean Dupont");
    let l1_id = make_lease(
        &conn,
        p1_id,
        t1_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    );
    make_payment(
        &conn,
        l1_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        "2024-01",
    );
    make_expense(
        &conn,
        p1_id,
        "taxe",
        3_000,
        NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
    );

    // Bien 2 : ni loyer ni dépense, doit quand même apparaître avec des zéros
    make_property(
        &conn,
        "Parking B3",
        "Rue du Marché 5",
        NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        1_000_000,
    );

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
    let p1_id = make_property(
        &conn,
        "Parking A12",
        "Rue de la Gare 10",
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_500_000,
    );
    let t1_id = make_tenant(&conn, "Jean Dupont");
    let l1_id = make_lease(
        &conn,
        p1_id,
        t1_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
    );
    make_payment(
        &conn,
        l1_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        "2024-01",
    );
}

#[test]
fn test_all_overdue_leases_mois_manquant() {
    let conn = db::open_in_memory().unwrap();

    // Bail avec un mois manquant
    let p2_id = make_property(
        &conn,
        "Parking B3",
        "Rue du Marché 5",
        NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
        1_200_000,
    );
    let t2_id = make_tenant(&conn, "Marie Leroy");
    let l2_id = make_lease(
        &conn,
        p2_id,
        t2_id,
        7_500,
        NaiveDate::from_ymd_opt(2023, 11, 1).unwrap(),
    );
    make_payment(
        &conn,
        l2_id,
        7_500,
        NaiveDate::from_ymd_opt(2023, 11, 5).unwrap(),
        "2023-11",
    );
}
