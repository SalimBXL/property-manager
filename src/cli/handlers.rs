use rusqlite::Connection;

use property_manager::db::reporting::{
    all_overdue_leases, all_properties_profitability, list_active_leases_with_names,
    list_expense_lines_for_property,
};
use property_manager::db::repository::{
    IndirectExpenseInput, delete_property, insert_expense, insert_indirect_expense, insert_lease,
    insert_property, insert_rent_payment, insert_tenant, list_properties, update_expense,
    update_property, update_tenant,
};
use property_manager::error::AppResult;
use property_manager::models::expense::Expense;
use property_manager::models::lease::Lease;
use property_manager::models::property::Property;
use property_manager::models::rent_payment::RentPayment;
use property_manager::models::tenant::Tenant;

use super::inputs::{
    AddExpenseInput, AddPropertyInput, IndirectExpenseArgs, UpdateExpenseInput, UpdatePropertyInput,
};
use super::{euros_to_cents, parse_date};

pub(crate) fn handle_add_indirect_expense(
    conn: &Connection,
    input: IndirectExpenseArgs,
) -> AppResult<()> {
    let count = input.properties.len();
    let id = insert_indirect_expense(
        conn,
        &IndirectExpenseInput {
            category: input.category,
            total_amount_cents: euros_to_cents(input.amount),
            expense_date: parse_date(&input.date)?,
            recurring: input.recurring,
            property_ids: input.properties,
        },
    )?;
    println!(
        "Frais indirect enregistré (id {}), réparti sur {} bien(s)",
        id, count
    );
    Ok(())
}

pub(crate) fn handle_list_active_leases(conn: &Connection) -> AppResult<()> {
    let leases = list_active_leases_with_names(conn)?;
    if leases.is_empty() {
        println!("Aucun bail actif.");
    }
    for l in leases {
        println!(
            "[bail {}] {} — locataire {} — loyer {:.2} €/mois — depuis le {}",
            l.lease_id,
            l.property_label,
            l.tenant_name,
            l.monthly_rent_cents as f64 / 100.0,
            l.start_date
        );
    }
    Ok(())
}

pub(crate) fn handle_list_expenses(conn: &Connection, property_id: i64) -> AppResult<()> {
    let expenses = list_expense_lines_for_property(conn, property_id)?;
    if expenses.is_empty() {
        println!("Aucune dépense enregistrée pour ce bien.");
    }
    for e in expenses {
        let tag = if e.is_indirect { " [indirect]" } else { "" };
        println!(
            "{} — {:.2} € — {}{}{}",
            e.category,
            e.allocated_amount_cents as f64 / 100.0,
            e.expense_date,
            if e.recurring { " (récurrente)" } else { "" },
            tag
        );
    }
    Ok(())
}

pub(crate) fn handle_delete_property(conn: &Connection, property_id: i64) {
    match delete_property(conn, property_id) {
        Ok(()) => println!("Bien {} supprimé.", property_id),
        Err(e) => println!("Suppression refusée : {}", e),
    }
}

pub(crate) fn handle_add_property(conn: &Connection, input: AddPropertyInput) -> AppResult<()> {
    let property = Property::new(
        input.label,
        input.address,
        parse_date(&input.purchase_date)?,
        euros_to_cents(input.purchase_price),
        input.notes,
    )?;
    let id = insert_property(conn, &property)?;
    println!("Bien créé avec l'id {}", id);
    Ok(())
}

pub(crate) fn handle_list_properties(conn: &Connection) -> AppResult<()> {
    let properties = list_properties(conn)?;
    if properties.is_empty() {
        println!("Aucun bien enregistré.");
    }
    for p in properties {
        println!(
            "[{}] {} — {} — acheté le {} pour {:.2} €",
            p.id().unwrap(),
            p.label(),
            p.address(),
            p.purchase_date(),
            p.purchase_price_cents() as f64 / 100.0
        );
    }
    Ok(())
}

pub(crate) fn handle_add_tenant(
    conn: &Connection,
    name: String,
    contact: Option<String>,
) -> AppResult<()> {
    let tenant = Tenant::new(name, contact);
    let id = insert_tenant(conn, &tenant)?;
    println!("Locataire créé avec l'id {}", id);
    Ok(())
}

pub(crate) fn handle_add_lease(
    conn: &Connection,
    property_id: i64,
    tenant_id: i64,
    monthly_rent: f64,
    start_date: String,
) -> AppResult<()> {
    let lease = Lease::new(
        property_id,
        tenant_id,
        euros_to_cents(monthly_rent),
        parse_date(&start_date)?,
        None,
    )?;
    let id = insert_lease(conn, &lease)?;
    println!("Bail créé avec l'id {}", id);
    Ok(())
}

pub(crate) fn handle_add_expense(conn: &Connection, input: AddExpenseInput) -> AppResult<()> {
    let expense = Expense::new_direct(
        input.property_id,
        input.category,
        euros_to_cents(input.amount),
        parse_date(&input.date)?,
        input.recurring,
    )?;
    let id = insert_expense(conn, &expense)?;
    println!("Dépense enregistrée avec l'id {}", id);
    Ok(())
}

pub(crate) fn handle_add_payment(
    conn: &Connection,
    lease_id: i64,
    amount: f64,
    date: String,
    period: String,
) -> AppResult<()> {
    let payment = RentPayment::new(lease_id, euros_to_cents(amount), parse_date(&date)?, period)?;
    let id = insert_rent_payment(conn, &payment)?;
    println!("Paiement enregistré avec l'id {}", id);
    Ok(())
}

pub(crate) fn handle_profitability(conn: &Connection) -> AppResult<()> {
    let results = all_properties_profitability(conn)?;
    if results.is_empty() {
        println!("Aucun bien enregistré.");
    }
    for r in results {
        println!(
            "{:<20} loyers: {:>10.2} €   dépenses: {:>10.2} €   net: {:>10.2} €",
            r.label,
            r.total_rent_collected as f64 / 100.0,
            r.total_expenses as f64 / 100.0,
            r.net_result as f64 / 100.0
        );
    }
    Ok(())
}

pub(crate) fn handle_overdue(conn: &Connection, up_to: Option<String>) -> AppResult<()> {
    let reference_date = match up_to {
        Some(s) => parse_date(&s)?,
        None => chrono::Local::now().date_naive(),
    };
    let overdue = all_overdue_leases(conn, reference_date)?;
    if overdue.is_empty() {
        println!("Aucun loyer en retard.");
    }
    for o in overdue {
        println!(
            "{} ({}) — mois manquants: {}",
            o.property_label,
            o.tenant_name,
            o.missing_months.join(", ")
        );
    }
    Ok(())
}

pub(crate) fn handle_update_property(
    conn: &Connection,
    input: UpdatePropertyInput,
) -> AppResult<()> {
    let property = Property::new(
        input.label,
        input.address,
        parse_date(&input.purchase_date)?,
        euros_to_cents(input.purchase_price),
        input.notes,
    )?;
    update_property(conn, input.property_id, &property)?;
    println!("Bien {} mis à jour.", input.property_id);
    Ok(())
}

pub(crate) fn handle_update_tenant(
    conn: &Connection,
    tenant_id: i64,
    name: String,
    contact: Option<String>,
) -> AppResult<()> {
    let tenant = Tenant::new(name, contact);
    update_tenant(conn, tenant_id, &tenant)?;
    println!("Locataire {} mis à jour.", tenant_id);
    Ok(())
}

pub(crate) fn handle_update_expense(conn: &Connection, input: UpdateExpenseInput) -> AppResult<()> {
    let expense = Expense::new_direct(
        input.property_id,
        input.category,
        euros_to_cents(input.amount),
        parse_date(&input.date)?,
        input.recurring,
    )?;
    update_expense(conn, input.expense_id, &expense)?;
    println!("Dépense {} mise à jour.", input.expense_id);
    Ok(())
}
