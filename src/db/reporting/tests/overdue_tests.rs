use chrono::NaiveDate;

use crate::db;
use crate::db::reporting::{all_overdue_leases, missing_rent_months};

use super::helpers::{make_lease, make_payment, make_property, make_tenant};

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
fn test_all_overdue_leases_excludes_lease_up_to_date() {
    let conn = db::open_in_memory().unwrap();

    // Bail à jour : ne doit pas remonter dans les loyers en retard
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

    let up_to = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
    let overdue = all_overdue_leases(&conn, up_to).unwrap();

    assert!(overdue.iter().all(|o| o.lease_id != l1_id));
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

    let up_to = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
    let overdue = all_overdue_leases(&conn, up_to).unwrap();

    assert_eq!(overdue.len(), 1);
    assert_eq!(overdue[0].lease_id, l2_id);
    assert_eq!(overdue[0].missing_months, vec!["2023-12".to_string()]);
}
