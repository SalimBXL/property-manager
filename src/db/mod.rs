// use rusqlite::{Connection, Result as SqlResult, params};
use rusqlite::Connection;
use std::path::Path;

pub mod reporting;
pub mod repository;
pub mod schema;

pub fn open(db_path: impl AsRef<Path>) -> rusqlite::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    schema::run_migrations(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", true)?;
    schema::run_migrations(&conn)?;
    Ok(conn)
}
