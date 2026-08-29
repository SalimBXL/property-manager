use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    create_property_table(conn)?;
    create_expense_table(conn)?;
    create_expense_allocation_table(conn)?;
    create_tenant_table(conn)?;
    create_lease_table(conn)?;
    create_rent_payment_table(conn)?;
    Ok(())
}

fn create_property_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS property (
            id INTEGER PRIMARY KEY,
            label TEXT NOT NULL,
            address TEXT NOT NULL,
            purchase_date TEXT NOT NULL,
            purchase_price_cents INTEGER NOT NULL,
            notes TEXT,
            CHECK (purchase_price_cents >= 0)
        );",
    )
}

fn create_expense_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS expense (
            id INTEGER PRIMARY KEY,
            property_id INTEGER REFERENCES property(id),
            category TEXT NOT NULL,
            amount_cents INTEGER NOT NULL,
            expense_date TEXT NOT NULL,
            recurring INTEGER NOT NULL DEFAULT 0,
            expense_type TEXT NOT NULL DEFAULT 'direct',
            CHECK (amount_cents >= 0),
            CHECK (
                (expense_type = 'direct' AND property_id IS NOT NULL)
                OR
                (expense_type = 'indirect' AND property_id IS NULL)
            )
        );",
    )
}

fn create_expense_allocation_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS expense_allocation (
            id INTEGER PRIMARY KEY,
            expense_id INTEGER NOT NULL REFERENCES expense(id),
            property_id INTEGER NOT NULL REFERENCES property(id),
            amount_cents INTEGER NOT NULL,
            CHECK (amount_cents >= 0)
        );",
    )
}

fn create_tenant_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS tenant (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            contact TEXT
        );",
    )
}

fn create_lease_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS lease (
            id INTEGER PRIMARY KEY,
            property_id INTEGER NOT NULL REFERENCES property(id),
            tenant_id INTEGER NOT NULL REFERENCES tenant(id),
            monthly_rent_cents INTEGER NOT NULL,
            start_date TEXT NOT NULL,
            end_date TEXT,
            CHECK (monthly_rent_cents >= 0),
            CHECK (end_date IS NULL OR end_date >= start_date)
        );
        CREATE UNIQUE INDEX IF NOT EXISTS
        idx_one_active_lease_per_property
        ON lease(property_id)
        WHERE end_date IS NULL;",
    )
}

fn create_rent_payment_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rent_payment (
            id INTEGER PRIMARY KEY,
            lease_id INTEGER NOT NULL REFERENCES lease(id),
            amount_cents INTEGER NOT NULL,
            payment_date TEXT NOT NULL,
            period_month TEXT NOT NULL,
            CHECK (amount_cents >= 0)
        );",
    )
}
