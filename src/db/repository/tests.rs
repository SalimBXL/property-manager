use super::*;
use crate::db;
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
fn test_indirect_expense_split_and_totals() {
    let conn = db::open_in_memory().unwrap();

    let p1 = Property::new(
        "Parking A".to_string(),
        "Rue A".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    )
    .unwrap();
    let p1_id = insert_property(&conn, &p1).unwrap();

    let p2 = Property::new(
        "Parking B".to_string(),
        "Rue B".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    )
    .unwrap();
    let p2_id = insert_property(&conn, &p2).unwrap();

    // Syndic de 100.01 € réparti sur 2 biens : 50.01 € et 50.00 €
    insert_indirect_expense(
        &conn,
        &IndirectExpenseInput {
            category: "syndic".to_string(),
            total_amount_cents: 10_001,
            expense_date: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
            recurring: true,
            property_ids: vec![p1_id, p2_id],
        },
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
    )
    .unwrap();

    let id = insert_property(&conn, &p).unwrap();
    let fetched = get_property(&conn, id).unwrap();

    assert_eq!(fetched.label(), "Parking A12");
    assert_eq!(fetched.purchase_price_cents(), 1_500_000);
}

#[test]
fn sql_check_constraint_rejects_direct_without_property() {
    let conn = db::open_in_memory().unwrap();
    let result = conn.execute(
        "INSERT INTO expense (property_id, category, amount_cents, expense_date, recurring, expense_type)
         VALUES (NULL, 'test', 1000, '2024-01-01', 0, 'direct')",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn sql_check_constraint_rejects_indirect_with_property() {
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

    let result = conn.execute(
        "INSERT INTO expense (property_id, category, amount_cents, expense_date, recurring, expense_type)
         VALUES (?1, 'test', 1000, '2024-01-01', 0, 'indirect')",
        params![property_id],
    );
    assert!(result.is_err());
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
    )
    .unwrap();
    let property_id = insert_property(&conn, &p).unwrap();

    let e1 = Expense::new_direct(
        property_id,
        "taxe foncière".to_string(),
        25_000,
        NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
        true,
    )
    .unwrap();
    let e2 = Expense::new_direct(
        property_id,
        "réparation".to_string(),
        8_000,
        NaiveDate::from_ymd_opt(2024, 9, 12).unwrap(),
        false,
    )
    .unwrap();

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

    let tenant = Tenant::new("Marc Petit".to_string(), None);
    let tenant_id = insert_tenant(&conn, &tenant).unwrap();

    let lease = Lease::new(
        property_id,
        tenant_id,
        6_000,
        NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(),
        None,
    )
    .unwrap();
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
    )
    .unwrap();
    let property_id = insert_property(&conn, &property).unwrap();

    delete_property(&conn, property_id).unwrap();

    let result = get_property(&conn, property_id);
    assert!(result.is_err());
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
fn indirect_expense_rejects_negative_total() {
    let conn = db::open_in_memory().unwrap();
    let p = Property::new(
        "Parking A".to_string(),
        "Rue A".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    )
    .unwrap();
    let property_id = insert_property(&conn, &p).unwrap();

    let result = insert_indirect_expense(
        &conn,
        &IndirectExpenseInput {
            category: "syndic".to_string(),
            total_amount_cents: -1000,
            expense_date: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
            recurring: false,
            property_ids: vec![property_id],
        },
    );

    assert!(matches!(result, Err(AppError::InvalidAmount(-1000))));
}

#[test]
fn indirect_expense_rejects_duplicate_property_ids() {
    let conn = db::open_in_memory().unwrap();
    let p1 = Property::new(
        "Parking A".to_string(),
        "Rue A".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    )
    .unwrap();
    let p1_id = insert_property(&conn, &p1).unwrap();

    let p2 = Property::new(
        "Parking B".to_string(),
        "Rue B".to_string(),
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        1_000_000,
        None,
    )
    .unwrap();
    let p2_id = insert_property(&conn, &p2).unwrap();

    let result = insert_indirect_expense(
        &conn,
        &IndirectExpenseInput {
            category: "syndic".to_string(),
            total_amount_cents: 9_000,
            expense_date: NaiveDate::from_ymd_opt(2024, 3, 1).unwrap(),
            recurring: false,
            property_ids: vec![p1_id, p2_id, p2_id],
        },
    );

    assert!(matches!(result, Err(AppError::DuplicatePropertyAllocation(id)) if id == p2_id));
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
