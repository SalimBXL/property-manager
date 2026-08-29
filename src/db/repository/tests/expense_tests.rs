use crate::db;
use crate::db::repository::{
    IndirectExpenseInput, insert_expense, insert_indirect_expense, insert_property,
    total_expenses_for_property,
};
use crate::error::AppError;
use crate::models::expense::Expense;
use crate::models::property::Property;
use chrono::NaiveDate;
use rusqlite::params;

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

    assert_eq!(total_p1 + total_p2, 10_001);
    assert_eq!(total_p1, 5_001);
    assert_eq!(total_p2, 5_000);
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
