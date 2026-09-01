use chrono::NaiveDate;
use rusqlite::Connection;

use crate::db::repository::{
    IndirectExpenseInput, insert_expense, insert_indirect_expense, insert_lease, insert_property,
    insert_rent_payment, insert_tenant,
};
use crate::error::AppResult;
use crate::models::expense::Expense;
use crate::models::lease::Lease;
use crate::models::property::Property;
use crate::models::rent_payment::RentPayment;
use crate::models::tenant::Tenant;

fn euros(e: f64) -> i64 {
    (e * 100.0).round() as i64
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("date de seed invalide")
}

/// Insère un jeu de données de démonstration : 4 biens, 3 locataires, des
/// baux actifs et un terminé, des loyers (avec un mois manquant pour
/// illustrer la détection de retard), et des dépenses directes/indirectes.
///
/// Appelée uniquement sur une base tout juste créée — voir `db::open`.
pub fn seed_demo_data(conn: &Connection) -> AppResult<()> {
    // ---------- Biens ----------
    let p1 = insert_property(
        conn,
        &Property::new(
            "Pacific 290".to_string(),
            "Rue Willems, 1210 Bruxelles".to_string(),
            date(2006, 1, 1),
            euros(12_000.0),
            None,
        )?,
    )?;

    let p2 = insert_property(
        conn,
        &Property::new(
            "Pacific 298".to_string(),
            "Rue Willems, 1210 Bruxelles".to_string(),
            date(2006, 1, 1),
            euros(12_000.0),
            None,
        )?,
    )?;

    let p3 = insert_property(
        conn,
        &Property::new(
            "Pacific 172".to_string(),
            "Rue Willems, 1210 Bruxelles".to_string(),
            date(2006, 1, 1),
            euros(12_000.0),
            None,
        )?,
    )?;

    let p4 = insert_property(
        conn,
        &Property::new(
            "Terrain Oignies".to_string(),
            "5670 Oignies-en-Thiérache".to_string(),
            date(2020, 3, 10),
            euros(20_000.0),
            None,
        )?,
    )?;

    let _ = p4; // volontairement sans bail : illustre le cas "bien vacant"

    // ---------- Locataires ----------
    let t1 = insert_tenant(conn, &Tenant::new("COLLARD Pascal".to_string(), None))?;
    let t2 = insert_tenant(conn, &Tenant::new("CRISAN Ana-Lucioa".to_string(), None))?;
    let t3 = insert_tenant(conn, &Tenant::new("CAUPIN Léonard".to_string(), None))?;

    // ---------- Baux ----------
    // Actif, loyers à jour
    let l1 = insert_lease(
        conn,
        &Lease::new(p1, t1, euros(95.0), date(2026, 1, 1), None)?,
    )?;

    // Actif, en retard sur le mois courant
    let l2 = insert_lease(
        conn,
        &Lease::new(p2, t2, euros(85.0), date(2026, 1, 1), None)?,
    )?;
    let l3 = insert_lease(
        conn,
        &Lease::new(p3, t3, euros(75.0), date(2026, 1, 1), None)?,
    )?;

    // Terminé — property4 n'a donc aucun bail actif
    insert_lease(
        conn,
        &Lease::new(
            p4,
            t3,
            euros(65.0),
            date(2020, 4, 1),
            Some(date(2023, 12, 31)),
        )?,
    )?;

    // ---------- Paiements de loyer ----------
    // L1 : à jour
    for (period, day) in [("2026-06", 3), ("2026-07", 2), ("2026-08", 4)] {
        let month: u32 = period[5..7].parse().expect("mois de seed invalide");
        insert_rent_payment(
            conn,
            &RentPayment::new(l1, euros(95.0), date(2026, month, day), period.to_string())?,
        )?;
    }

    // L2 : juillet payé, août manquant -> remonte dans les loyers en retard
    insert_rent_payment(
        conn,
        &RentPayment::new(l2, euros(85.0), date(2026, 7, 5), "2026-07".to_string())?,
    )?;

    // L3 : juillet payé, août manquant -> remonte dans les loyers en retard
    insert_rent_payment(
        conn,
        &RentPayment::new(l3, euros(75.0), date(2026, 7, 5), "2026-07".to_string())?,
    )?;

    // ---------- Dépenses ----------
    insert_expense(
        conn,
        &Expense::new_direct(
            p1,
            "taxe foncière".to_string(),
            euros(250.0),
            date(2026, 1, 15),
            true,
        )?,
    )?;

    insert_expense(
        conn,
        &Expense::new_direct(
            p2,
            "réparation barrière".to_string(),
            euros(0.0),
            date(2026, 3, 10),
            false,
        )?,
    )?;

    insert_indirect_expense(
        conn,
        &IndirectExpenseInput {
            category: "syndic".to_string(),
            total_amount_cents: euros(495.24),
            expense_date: date(2026, 7, 1),
            recurring: true,
            property_ids: vec![p1, p2, p3],
        },
    )?;

    Ok(())
}
