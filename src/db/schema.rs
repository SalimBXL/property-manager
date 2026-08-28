use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS property (
            id INTEGER PRIMARY KEY,
            label TEXT NOT NULL,
            address TEXT NOT NULL,
            purchase_date TEXT NOT NULL,
            purchase_price_cents INTEGER NOT NULL,
            notes TEXT
        );

        CREATE TABLE IF NOT EXISTS expense (
            id INTEGER PRIMARY KEY,
            property_id INTEGER REFERENCES property(id), -- NULL si frais indirect
            category TEXT NOT NULL,
            amount_cents INTEGER NOT NULL,   -- montant TOTAL du frais
            expense_date TEXT NOT NULL,
            recurring INTEGER NOT NULL DEFAULT 0,
            expense_type TEXT NOT NULL DEFAULT 'direct' -- 'direct' | 'indirect'
        );

        CREATE TABLE IF NOT EXISTS expense_allocation (
            id INTEGER PRIMARY KEY,
            expense_id INTEGER NOT NULL REFERENCES expense(id),
            property_id INTEGER NOT NULL REFERENCES property(id),
            amount_cents INTEGER NOT NULL  -- part de ce bien dans le frais indirect
        );

        CREATE TABLE IF NOT EXISTS tenant (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            contact TEXT
        );

        CREATE TABLE IF NOT EXISTS lease (
            id INTEGER PRIMARY KEY,
            property_id INTEGER NOT NULL REFERENCES property(id),
            tenant_id INTEGER NOT NULL REFERENCES tenant(id),
            monthly_rent_cents INTEGER NOT NULL,
            start_date TEXT NOT NULL,
            end_date TEXT
        );

        CREATE TABLE IF NOT EXISTS rent_payment (
            id INTEGER PRIMARY KEY,
            lease_id INTEGER NOT NULL REFERENCES lease(id),
            amount_cents INTEGER NOT NULL,
            payment_date TEXT NOT NULL,
            period_month TEXT NOT NULL
        );
        ",
    )
}
