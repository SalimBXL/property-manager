use chrono::NaiveDate;
use property_manager::db;
use property_manager::db::reporting::*;
use property_manager::db::repository::*;
use property_manager::models::expense::Expense;
use property_manager::models::lease::Lease;
use property_manager::models::property::Property;
use property_manager::models::rent_payment::RentPayment;
use property_manager::models::tenant::Tenant;

/// Scénario avec 3 biens dans des situations différentes :
/// - un bien rentable et à jour de ses loyers
/// - un bien avec un locataire en retard
/// - un bien tout juste acheté, sans locataire ni activité
///
/// Ce test vérifie que les vues globales (`all_properties_profitability`,
/// `all_overdue_leases`) traitent correctement ce mélange, notamment le
/// bien sans aucune donnée liée (cas le plus fragile du LEFT JOIN).
#[test]
fn portfolio_overview_with_mixed_situations() {
    let conn = db::open_in_memory().unwrap();

    // ---------- Bien 1 : rentable, à jour ----------
    let p1 = Property::new(
        "Parking A12".to_string(),
        "Rue de la Gare 10".to_string(),
        NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
        1_500_000,
        None,
    )
    .unwrap();
    let p1_id = insert_property(&conn, &p1).unwrap();

    let t1 = Tenant::new("Jean Dupont".to_string(), None);
    let t1_id = insert_tenant(&conn, &t1).unwrap();

    let l1 = Lease::new(
        p1_id,
        t1_id,
        8_000,
        NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(),
        None,
    )
    .unwrap();
    let l1_id = insert_lease(&conn, &l1).unwrap();

    for (month, date) in [
        ("2023-01", NaiveDate::from_ymd_opt(2023, 1, 3).unwrap()),
        ("2023-02", NaiveDate::from_ymd_opt(2023, 2, 3).unwrap()),
        ("2023-03", NaiveDate::from_ymd_opt(2023, 3, 3).unwrap()),
    ] {
        insert_rent_payment(
            &conn,
            &RentPayment::new(l1_id, 8_000, date, month.to_string()).unwrap(),
        )
        .unwrap();
    }

    insert_expense(
        &conn,
        &Expense::new_direct(
            p1_id,
            "taxe foncière".to_string(),
            4_000,
            NaiveDate::from_ymd_opt(2023, 2, 15).unwrap(),
            true,
        )
        .unwrap(),
    )
    .unwrap();

    // ---------- Bien 2 : locataire en retard ----------
    let p2 = Property::new(
        "Parking A12".to_string(),
        "Rue de la Gare 10".to_string(),
        NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
        1_500_000,
        None,
    )
    .unwrap();
    let p2_id = insert_property(&conn, &p2).unwrap();

    let t2 = Tenant::new("Marie Leroy".to_string(), None);
    let t2_id = insert_tenant(&conn, &t2).unwrap();

    let l2 = Lease::new(
        p2_id,
        t2_id,
        7_500,
        NaiveDate::from_ymd_opt(2022, 11, 1).unwrap(),
        None,
    )
    .unwrap();
    let l2_id = insert_lease(&conn, &l2).unwrap();

    // Novembre et décembre payés, janvier oublié
    insert_rent_payment(
        &conn,
        &RentPayment::new(
            l2_id,
            7_500,
            NaiveDate::from_ymd_opt(2022, 11, 5).unwrap(),
            "2022-11".to_string(),
        )
        .unwrap(),
    )
    .unwrap();
    insert_rent_payment(
        &conn,
        &RentPayment::new(
            l2_id,
            7_500,
            NaiveDate::from_ymd_opt(2022, 12, 4).unwrap(),
            "2022-12".to_string(),
        )
        .unwrap(),
    )
    .unwrap();

    // ---------- Bien 3 : acheté récemment, sans locataire ----------
    let p3 = Property::new(
        "Parking B3".to_string(),
        "Rue de la Gare 10".to_string(),
        NaiveDate::from_ymd_opt(2024, 3, 15).unwrap(),
        1_500_000,
        None,
    )
    .unwrap();
    let p3_id = insert_property(&conn, &p3).unwrap();

    // ---------- Vérification : rentabilité globale ----------
    let profitability = all_properties_profitability(&conn).unwrap();
    assert_eq!(profitability.len(), 3);

    let r1 = profitability
        .iter()
        .find(|r| r.property_id == p1_id)
        .unwrap();
    assert_eq!(r1.total_rent_collected, 24_000);
    assert_eq!(r1.total_expenses, 4_000);
    assert_eq!(r1.net_result, 20_000);

    let r2 = profitability
        .iter()
        .find(|r| r.property_id == p2_id)
        .unwrap();
    assert_eq!(r2.total_rent_collected, 15_000);
    assert_eq!(r2.total_expenses, 0);
    assert_eq!(r2.net_result, 15_000);

    let r3 = profitability
        .iter()
        .find(|r| r.property_id == p3_id)
        .unwrap();
    assert_eq!(r3.total_rent_collected, 0);
    assert_eq!(r3.total_expenses, 0);
    assert_eq!(r3.net_result, 0);

    // ---------- Vérification : loyers en retard ----------
    let up_to = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();
    let overdue = all_overdue_leases(&conn, up_to).unwrap();

    // Seul le bail du bien 2 doit apparaître : bien 1 à jour, bien 3 sans bail du tout
    assert_eq!(overdue.len(), 1);
    assert_eq!(overdue[0].lease_id, l2_id);
    assert_eq!(overdue[0].property_label, "Parking A12");
    assert_eq!(overdue[0].tenant_name, "Marie Leroy");
    assert_eq!(overdue[0].missing_months, vec!["2023-01".to_string()]);
}

/// Cas limite : aucun bien en base du tout. Les vues globales doivent
/// renvoyer des listes vides plutôt qu'une erreur.
#[test]
fn portfolio_overview_with_no_properties() {
    let conn = db::open_in_memory().unwrap();

    let profitability = all_properties_profitability(&conn).unwrap();
    assert!(profitability.is_empty());

    let up_to = NaiveDate::from_ymd_opt(2023, 1, 31).unwrap();
    let overdue = all_overdue_leases(&conn, up_to).unwrap();
    assert!(overdue.is_empty());
}
