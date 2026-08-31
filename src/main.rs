mod cli;
mod tui;

use clap::Parser;

use property_manager::db;
use property_manager::error::AppResult;

use cli::{Cli, Command};

fn main() -> AppResult<()> {
    let args = Cli::parse();
    let conn = db::open(&args.db_path)?;

    // Cas particulier : le dashboard gère sa propre connexion en interne
    // et prend le contrôle du terminal, donc on le sort du dispatch commun.
    if let Command::Dashboard = args.command {
        drop(conn);
        tui::run(&args.db_path)?;
        return Ok(());
    }

    cli::run_command(&conn, args.command)
}
