use chrono::NaiveDate;
use rusqlite::Connection;

use crate::db::repository::{
    insert_expense, insert_lease, insert_property, insert_rent_payment, insert_tenant,
};
use crate::models::expense::Expense;
use crate::models::lease::Lease;
use crate::models::property::Property;
use crate::models::rent_payment::RentPayment;
use crate::models::tenant::Tenant;

/// Crée un bien de test et retourne son id, avec `.unwrap()` déjà géré.
pub(super) fn make_property(
    conn: &Connection,
    label: &str,
    address: &str,
    purchase_date: NaiveDate,
    price_cents: i64,
) -> i64 {
    let p = Property::new(
        label.to_string(),
        address.to_string(),
        purchase_date,
        price_cents,
        None,
    )
    .unwrap();
    insert_property(conn, &p).unwrap()
}

/// Crée un locataire de test et retourne son id.
pub(super) fn make_tenant(conn: &Connection, name: &str) -> i64 {
    insert_tenant(conn, &Tenant::new(name.to_string(), None)).unwrap()
}

/// Crée un bail actif de test et retourne son id.
pub(super) fn make_lease(
    conn: &Connection,
    property_id: i64,
    tenant_id: i64,
    monthly_rent_cents: i64,
    start_date: NaiveDate,
) -> i64 {
    let l = Lease::new(property_id, tenant_id, monthly_rent_cents, start_date, None).unwrap();
    insert_lease(conn, &l).unwrap()
}

/// Enregistre un paiement de loyer de test.
pub(super) fn make_payment(
    conn: &Connection,
    lease_id: i64,
    amount_cents: i64,
    date: NaiveDate,
    period: &str,
) {
    let rp = RentPayment::new(lease_id, amount_cents, date, period.to_string()).unwrap();
    insert_rent_payment(conn, &rp).unwrap();
}

/// Enregistre une dépense directe de test.
pub(super) fn make_expense(
    conn: &Connection,
    property_id: i64,
    category: &str,
    amount_cents: i64,
    date: NaiveDate,
) {
    let e =
        Expense::new_direct(property_id, category.to_string(), amount_cents, date, true).unwrap();
    insert_expense(conn, &e).unwrap();
}
