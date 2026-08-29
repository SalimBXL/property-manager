use crate::db;
use crate::db::repository::{delete_property, get_property, insert_property};
use crate::error::AppError;
use crate::models::property::Property;
use chrono::NaiveDate;

#[test]
fn property_rejects_negative_purchase_price() {
    let result = Property::new(
        "Parking Invalide".to_string(),
        "Rue Test".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        -500_000,
        None,
    );
    assert!(matches!(result, Err(AppError::InvalidAmount(-500_000))));
}

#[test]
fn test_insert_and_get_property() {
    let conn = db::open_in_memory().unwrap();
    let p = Property::new(
        "Parking A12".to_string(),
        "Rue de la Gare 10".to_string(),
        NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
        1_500_000,
        None,
    )
    .unwrap();

    let id = insert_property(&conn, &p).unwrap();
    let fetched = get_property(&conn, id).unwrap();

    assert_eq!(fetched.label(), "Parking A12");
    assert_eq!(fetched.purchase_price_cents(), 1_500_000);
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
    )
    .unwrap();
    let property_id = insert_property(&conn, &property).unwrap();

    let tenant = crate::models::tenant::Tenant::new("Marc Petit".to_string(), None);
    let tenant_id = crate::db::repository::insert_tenant(&conn, &tenant).unwrap();

    let lease = crate::models::lease::Lease::new(
        property_id,
        tenant_id,
        6_000,
        NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
        None,
    )
    .unwrap();
    crate::db::repository::insert_lease(&conn, &lease).unwrap();

    let result = delete_property(&conn, property_id);
    assert!(matches!(result, Err(AppError::PropertyHasDependents(id)) if id == property_id));

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
    )
    .unwrap();
    let property_id = insert_property(&conn, &property).unwrap();

    delete_property(&conn, property_id).unwrap();

    let result = get_property(&conn, property_id);
    assert!(result.is_err());
}
