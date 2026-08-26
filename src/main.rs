use property_manager::db;
use property_manager::error::AppResult;

fn main() -> AppResult<()> {
    let conn = db::open("property_manager.db")?;
    println!("Base de données initialisée avec succès.");
    let _ = conn;
    Ok(())
}
