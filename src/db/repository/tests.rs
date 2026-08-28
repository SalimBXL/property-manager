use super::*;
use crate::db;
use chrono::NaiveDate;

#[test]
fn test_indirect_expense_split_and_totals() {
    let conn = db::open_in_memory().unwrap();

    let p1 = Property::new(
        "Parking A".to_string(),
        "Rue A".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    );
    let p1_id = insert_property(&conn, &p1).unwrap();

    let p2 = Property::new(
        "Parking B".to_string(),
        "Rue B".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    );
    let p2_id = insert_property(&conn, &p2).unwrap();

    // Syndic de 100.01 € réparti sur 2 biens : 50.01 € et 50.00 €
    insert_indirect_expense(
        &conn,
        "syndic",
        10_001,
        NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
        true,
        &[p1_id, p2_id],
    )
    .unwrap();

    let total_p1 = total_expenses_for_property(&conn, p1_id).unwrap();
    let total_p2 = total_expenses_for_property(&conn, p2_id).unwrap();

    // La somme des deux parts doit reconstituer exactement le total, centime près
    assert_eq!(total_p1 + total_p2, 10_001);
    // Le reste (1 centime) va à la première propriété de la liste
    assert_eq!(total_p1, 5_001);
    assert_eq!(total_p2, 5_000);
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
