use chrono::NaiveDate;

use crate::db;
use crate::db::reporting::{all_properties_profitability, property_profitability};

use super::helpers::{make_expense, make_lease, make_payment, make_property, make_tenant};

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
