mod cli;
mod db;
mod errors;
mod models;
mod output;
mod recur;

use clap::Parser;
use std::process;

use cli::{Cli, Commands};
use db::Database;
use errors::FlowstateError;

fn db_path() -> String {
    ".flowstate.db".to_string()
}

fn run() -> Result<(), FlowstateError> {
    let cli = Cli::parse();

    // Allow overriding DB path via env var (useful for testing)
    let path = std::env::var("FLOWSTATE_DB").unwrap_or_else(|_| db_path());
    let db = Database::open(&path)?;

    match cli.command {
        Commands::Task { action } => cli::task::handle(action, &db),
        Commands::Agenda { date, json } => cli::agenda::handle_agenda(&db, date, json),
        Commands::Overdue { json } => cli::agenda::handle_overdue(&db, json),
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        process::exit(e.exit_code());
    }
}
