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
}

#[test]
fn test_all_overdue_leases_mois_manquant() {
    let conn = db::open_in_memory().unwrap();

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
}
