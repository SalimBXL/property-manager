use crate::db;
use crate::db::repository::{
    active_lease_for_property, insert_lease, insert_property, insert_tenant,
};
use crate::error::AppError;
use crate::models::lease::Lease;
use crate::models::property::Property;
use crate::models::tenant::Tenant;
use chrono::NaiveDate;
use rusqlite::params;

#[test]
fn test_insert_lease_with_tenant() {
    let conn = db::open_in_memory().unwrap();

    let p = Property::new(
        "Parking A12".to_string(),
        "Rue de la Gare 10".to_string(),
        NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
        1_500_000,
        None,
    )
    .unwrap();
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
    )
    .unwrap();
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
    )
    .unwrap();
    let property_id = insert_property(&conn, &p).unwrap();

    let t = Tenant::new("Marie Leroy".to_string(), None);
    let tenant_id = insert_tenant(&conn, &t).unwrap();

    let l = Lease::new(
        property_id,
        tenant_id,
        7_500,
        NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
        Some(NaiveDate::from_ymd_opt(2024, 1, 31).unwrap()),
    )
    .unwrap();
    insert_lease(&conn, &l).unwrap();

    let active = active_lease_for_property(&conn, property_id).unwrap();
    assert!(active.is_none());
}

#[test]
fn lease_rejects_negative_monthly_rent() {
    let result = Lease::new(
        1,
        1,
        -5_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    );
    assert!(matches!(result, Err(AppError::InvalidAmount(-5_000))));
}

#[test]
fn cannot_insert_two_active_leases_for_same_property() {
    let conn = db::open_in_memory().unwrap();
    let p = Property::new(
        "Parking A12".to_string(),
        "Rue de la Gare 10".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    )
    .unwrap();
    let property_id = insert_property(&conn, &p).unwrap();

    let t1 = Tenant::new("Jean Dupont".to_string(), None);
    let t1_id = insert_tenant(&conn, &t1).unwrap();
    let l1 = Lease::new(
        property_id,
        t1_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    )
    .unwrap();
    insert_lease(&conn, &l1).unwrap();

    let t2 = Tenant::new("Marie Leroy".to_string(), None);
    let t2_id = insert_tenant(&conn, &t2).unwrap();
    let l2 = Lease::new(
        property_id,
        t2_id,
        7_500,
        NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        None,
    )
    .unwrap();

    let result = insert_lease(&conn, &l2);
    assert!(matches!(
        result,
        Err(AppError::PropertyAlreadyHasActiveLease(id)) if id == property_id
    ));
}

#[test]
fn can_insert_new_active_lease_after_previous_one_ended() {
    let conn = db::open_in_memory().unwrap();
    let p = Property::new(
        "Parking B3".to_string(),
        "Rue du Marché 5".to_string(),
        NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
        1_000_000,
        None,
    )
    .unwrap();
    let property_id = insert_property(&conn, &p).unwrap();

    let t1 = Tenant::new("Ancien Locataire".to_string(), None);
    let t1_id = insert_tenant(&conn, &t1).unwrap();
    let l1 = Lease::new(
        property_id,
        t1_id,
        6_000,
        NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
        Some(NaiveDate::from_ymd_opt(2023, 12, 31).unwrap()),
    )
    .unwrap();
    insert_lease(&conn, &l1).unwrap();

    let t2 = Tenant::new("Nouveau Locataire".to_string(), None);
    let t2_id = insert_tenant(&conn, &t2).unwrap();
    let l2 = Lease::new(
        property_id,
        t2_id,
        6_500,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    )
    .unwrap();

    let result = insert_lease(&conn, &l2);
    assert!(result.is_ok());
}

#[test]
fn lease_rejects_end_date_before_start_date() {
    let result = Lease::new(
        1,
        1,
        5_000,
        NaiveDate::from_ymd_opt(2026, 8, 20).unwrap(),
        Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()),
    );
    assert!(matches!(result, Err(AppError::InvalidLeaseDates { .. })));
}

#[test]
fn lease_accepts_end_date_after_start_date() {
    let result = Lease::new(
        1,
        1,
        5_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
    );
    assert!(result.is_ok());
}

#[test]
fn sql_check_constraint_rejects_end_date_before_start_date() {
    let conn = db::open_in_memory().unwrap();
    let p = Property::new(
        "Parking Test".to_string(),
        "Rue Test".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    )
    .unwrap();
    let property_id = insert_property(&conn, &p).unwrap();
    let t = Tenant::new("Test Tenant".to_string(), None);
    let tenant_id = insert_tenant(&conn, &t).unwrap();

    let result = conn.execute(
        "INSERT INTO lease (property_id, tenant_id, monthly_rent_cents, start_date, end_date)
         VALUES (?1, ?2, 5000, '2026-08-20', '2026-01-01')",
        params![property_id, tenant_id],
    );
    assert!(result.is_err());
}
