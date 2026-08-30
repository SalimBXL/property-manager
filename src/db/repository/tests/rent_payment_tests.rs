use crate::db;
use crate::db::repository::{
    insert_lease, insert_property, insert_rent_payment, insert_tenant, total_paid_for_lease,
};
use crate::error::AppError;
use crate::models::lease::Lease;
use crate::models::property::Property;
use crate::models::rent_payment::RentPayment;
use crate::models::tenant::Tenant;
use chrono::NaiveDate;
use rusqlite::params;

#[test]
fn test_rent_payment_total() {
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

    let t = Tenant::new("Jean Dupont".to_string(), None);
    let tenant_id = insert_tenant(&conn, &t).unwrap();

    let l = Lease::new(
        property_id,
        tenant_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 4, 1).unwrap(),
        None,
    )
    .unwrap();
    let lease_id = insert_lease(&conn, &l).unwrap();

    let rp1 = RentPayment::new(
        lease_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 4, 3).unwrap(),
        "2024-04".to_string(),
    )
    .unwrap();
    let rp2 = RentPayment::new(
        lease_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 5, 2).unwrap(),
        "2024-05".to_string(),
    )
    .unwrap();

    insert_rent_payment(&conn, &rp1).unwrap();
    insert_rent_payment(&conn, &rp2).unwrap();

    let total = total_paid_for_lease(&conn, lease_id).unwrap();
    assert_eq!(total, 16_000);
}

#[test]
fn sql_check_constraint_rejects_negative_rent_payment() {
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
    let l = Lease::new(
        property_id,
        tenant_id,
        5_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    )
    .unwrap();
    let lease_id = insert_lease(&conn, &l).unwrap();

    let result = conn.execute(
        "INSERT INTO rent_payment (lease_id, amount_cents, payment_date, period_month)
         VALUES (?1, -100, '2024-01-01', '2024-01')",
        params![lease_id],
    );
    assert!(result.is_err());
}

#[test]
fn rent_payment_rejects_negative_amount() {
    let result = RentPayment::new(
        1,
        -100,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        "2024-01".to_string(),
    );
    assert!(matches!(result, Err(AppError::InvalidAmount(-100))));
}

#[test]
fn rent_payment_rejects_malformed_period_month() {
    let result = RentPayment::new(
        1,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        "banane".to_string(),
    );
    assert!(matches!(result, Err(AppError::InvalidPeriodMonth(_))));
}

#[test]
fn rent_payment_rejects_out_of_range_month() {
    let result = RentPayment::new(
        1,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        "2024-13".to_string(),
    );
    assert!(matches!(result, Err(AppError::InvalidPeriodMonth(_))));
}

#[test]
fn rent_payment_accepts_valid_period_month() {
    let result = RentPayment::new(
        1,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        "2024-01".to_string(),
    );
    assert!(result.is_ok());
}

#[test]
fn sql_check_constraint_rejects_malformed_period_month() {
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
    let l = Lease::new(
        property_id,
        tenant_id,
        5_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    )
    .unwrap();
    let lease_id = insert_lease(&conn, &l).unwrap();

    let result = conn.execute(
        "INSERT INTO rent_payment (lease_id, amount_cents, payment_date, period_month)
         VALUES (?1, 8000, '2024-01-01', '2024-13')",
        params![lease_id],
    );
    assert!(result.is_err());
}

#[test]
fn cannot_insert_two_payments_for_same_lease_and_period() {
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
    let t = Tenant::new("Jean Dupont".to_string(), None);
    let tenant_id = insert_tenant(&conn, &t).unwrap();
    let l = Lease::new(
        property_id,
        tenant_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    )
    .unwrap();
    let lease_id = insert_lease(&conn, &l).unwrap();

    let rp1 = RentPayment::new(
        lease_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        "2024-01".to_string(),
    )
    .unwrap();
    insert_rent_payment(&conn, &rp1).unwrap();

    let rp2 = RentPayment::new(
        lease_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 15).unwrap(), // date différente, même période
        "2024-01".to_string(),
    )
    .unwrap();

    let result = insert_rent_payment(&conn, &rp2);
    assert!(matches!(
        result,
        Err(AppError::DuplicateRentPayment { lease_id: lid, period }) if lid == lease_id && period == "2024-01"
    ));
}

#[test]
fn can_insert_payments_for_different_periods() {
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
    let t = Tenant::new("Jean Dupont".to_string(), None);
    let tenant_id = insert_tenant(&conn, &t).unwrap();
    let l = Lease::new(
        property_id,
        tenant_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        None,
    )
    .unwrap();
    let lease_id = insert_lease(&conn, &l).unwrap();

    let rp1 = RentPayment::new(
        lease_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 1, 3).unwrap(),
        "2024-01".to_string(),
    )
    .unwrap();
    let rp2 = RentPayment::new(
        lease_id,
        8_000,
        NaiveDate::from_ymd_opt(2024, 2, 3).unwrap(),
        "2024-02".to_string(),
    )
    .unwrap();

    assert!(insert_rent_payment(&conn, &rp1).is_ok());
    assert!(insert_rent_payment(&conn, &rp2).is_ok());
}
