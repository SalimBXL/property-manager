use rusqlite::Connection;
use std::path::Path;

pub mod reporting;
pub mod repository;
pub mod schema;
mod seed;

use crate::error::AppResult;

pub fn open(db_path: impl AsRef<Path>) -> AppResult<Connection> {
    let is_new = !db_path.as_ref().exists();

    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    schema::run_migrations(&conn)?;

    if is_new {
        seed::seed_demo_data(&conn)?;
    }

    Ok(conn)
}

pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", true)?;
    schema::run_migrations(&conn)?;
    Ok(conn)
}
