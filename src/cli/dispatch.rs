use rusqlite::Connection;

use property_manager::error::AppResult;

use super::Command;
use super::handlers::{
    handle_add_expense, handle_add_indirect_expense, handle_add_lease, handle_add_payment,
    handle_add_property, handle_add_tenant, handle_delete_property, handle_list_active_leases,
    handle_list_expenses, handle_list_properties, handle_overdue, handle_profitability,
    handle_update_expense, handle_update_property, handle_update_tenant,
};
use super::inputs::{
    AddExpenseInput, AddPropertyInput, IndirectExpenseArgs, UpdateExpenseInput, UpdatePropertyInput,
};

pub fn run_command(conn: &Connection, command: Command) -> AppResult<()> {
    match command {
        Command::Dashboard => unreachable!("géré en amont dans main()"),

        Command::ListActiveLeases => handle_list_active_leases(conn)?,
        Command::ListExpenses { property_id } => handle_list_expenses(conn, property_id)?,
        Command::DeleteProperty { property_id } => handle_delete_property(conn, property_id),
        Command::ListProperties => handle_list_properties(conn)?,
        Command::AddTenant { name, contact } => handle_add_tenant(conn, name, contact)?,

        Command::AddLease {
            property_id,
            tenant_id,
            monthly_rent,
            start_date,
        } => handle_add_lease(conn, property_id, tenant_id, monthly_rent, start_date)?,

        Command::AddPayment {
            lease_id,
            amount,
            date,
            period,
        } => handle_add_payment(conn, lease_id, amount, date, period)?,

        Command::Profitability => handle_profitability(conn)?,
        Command::Overdue { up_to } => handle_overdue(conn, up_to)?,

        cmd @ (Command::UpdateProperty { .. }
        | Command::UpdateTenant { .. }
        | Command::UpdateExpense { .. }) => run_update_command(conn, cmd)?,

        other => run_add_command(conn, other)?,
    }
    Ok(())
}

fn run_add_command(conn: &Connection, command: Command) -> AppResult<()> {
    match command {
        Command::AddIndirectExpense {
            category,
            amount,
            date,
            properties,
            recurring,
        } => handle_add_indirect_expense(
            conn,
            IndirectExpenseArgs::new(category, amount, date, properties, recurring),
        ),

        Command::AddProperty {
            label,
            address,
            purchase_date,
            purchase_price,
            notes,
        } => handle_add_property(
            conn,
            AddPropertyInput::new(label, address, purchase_date, purchase_price, notes),
        ),

        Command::AddExpense {
            property_id,
            category,
            amount,
            date,
            recurring,
        } => handle_add_expense(
            conn,
            AddExpenseInput::new(property_id, category, amount, date, recurring),
        ),

        _ => unreachable!("géré en amont dans run_command()"),
    }
}

fn run_update_command(conn: &Connection, command: Command) -> AppResult<()> {
    match command {
        Command::UpdateProperty {
            property_id,
            label,
            address,
            purchase_date,
            purchase_price,
            notes,
        } => handle_update_property(
            conn,
            UpdatePropertyInput::new(
                property_id,
                AddPropertyInput::new(label, address, purchase_date, purchase_price, notes),
            ),
        )?,

        Command::UpdateTenant {
            tenant_id,
            name,
            contact,
        } => handle_update_tenant(conn, tenant_id, name, contact)?,

        Command::UpdateExpense {
            expense_id,
            property_id,
            category,
            amount,
            date,
            recurring,
        } => handle_update_expense(
            conn,
            UpdateExpenseInput::new(
                expense_id,
                AddExpenseInput::new(property_id, category, amount, date, recurring),
            ),
        )?,

        _ => unreachable!("géré ailleurs"),
    }
    Ok(())
}
