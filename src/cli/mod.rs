pub mod agenda;
pub mod task;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "flowstate", about = "Task management CLI for AI agents")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage tasks
    Task {
        #[command(subcommand)]
        action: task::TaskAction,
    },
    /// Show today's agenda
    Agenda {
        /// Target date (YYYY-MM-DD), defaults to today
        #[arg(long)]
        date: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show overdue tasks
    Overdue {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}
