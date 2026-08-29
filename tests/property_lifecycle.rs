use chrono::NaiveDate;
use property_manager::db;
use property_manager::db::repository::*;
use property_manager::models::expense::Expense;
use property_manager::models::lease::Lease;
use property_manager::models::property::Property;
use property_manager::models::rent_payment::RentPayment;
use property_manager::models::tenant::Tenant;

#[test]
fn full_property_lifecycle() {
    let conn = db::open_in_memory().unwrap();

    // 1. Achat d'un bien
    let property = Property::new(
        "Parking C7".to_string(),
        "Rue de la Gare 10".to_string(),
        NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
        1_500_000,
        None,
    )
    .unwrap();
    let property_id = insert_property(&conn, &property).unwrap();

    // 2. Un locataire arrive, bail signé
    let tenant = Tenant::new(
        "Sophie Bernard".to_string(),
        Some("sophie@example.com".to_string()),
    );
    let tenant_id = insert_tenant(&conn, &tenant).unwrap();

    let lease = Lease::new(
        property_id,
        tenant_id,
        9_000,
        NaiveDate::from_ymd_opt(2024, 2, 1).unwrap(),
        None,
    );
    let lease_id = insert_lease(&conn, &lease).unwrap();

    let active = active_lease_for_property(&conn, property_id).unwrap();
    assert!(active.is_some());

    // 3. Quelques mois de loyers encaissés
    for (month, date) in [
        ("2024-02", NaiveDate::from_ymd_opt(2024, 2, 3).unwrap()),
        ("2024-03", NaiveDate::from_ymd_opt(2024, 3, 4).unwrap()),
        ("2024-04", NaiveDate::from_ymd_opt(2024, 4, 2).unwrap()),
    ] {
        insert_rent_payment(
            &conn,
            &RentPayment::new(lease_id, 9_000, date, month.to_string()),
        )
        .unwrap();
    }

    // 4. Une dépense imprévue
    insert_expense(
        &conn,
        &Expense::new(
            property_id,
            "réparation barrière".to_string(),
            15_000,
            NaiveDate::from_ymd_opt(2024, 3, 10).unwrap(),
            false,
        ),
    )
    .unwrap();

    // 5. Vérifications finales : le bien existe bien avec les bonnes données
    let fetched = get_property(&conn, property_id).unwrap();
    assert_eq!(fetched.label(), "Parking C7");

    let total_rent = total_paid_for_lease(&conn, lease_id).unwrap();
    assert_eq!(total_rent, 27_000);

    let total_expenses = total_expenses_for_property(&conn, property_id).unwrap();
    assert_eq!(total_expenses, 15_000);
}
