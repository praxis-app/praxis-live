use clap::{Args, Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "praxis-live-cli",
    version,
    about = "Read-only developer utilities for Praxis Live",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Poll and proposal stats with vote breakdown.
    PollStats(PollStatsArgs),
    /// Print the current database schema.
    Schema,
    /// Print all Axum API routes extracted from route files.
    Routes(RoutesArgs),
}

#[derive(Args, Debug)]
pub struct PollStatsArgs {
    /// Lookback window in days.
    #[arg(long, default_value_t = 30)]
    pub days: i64,
    /// Optional server scope.
    #[arg(long)]
    pub server_id: Option<Uuid>,
    /// Optional channel scope.
    #[arg(long)]
    pub channel_id: Option<Uuid>,
    /// Focus on a specific poll ID.
    #[arg(long)]
    pub poll_id: Option<Uuid>,
    /// How many high-participation polls to list.
    #[arg(long, default_value_t = 5)]
    pub top_polls: i64,
    /// Limit of top channels to display.
    #[arg(long, default_value_t = 5)]
    pub top_channels: i64,
}

#[derive(Args, Debug)]
pub struct RoutesArgs {
    /// Filter routes by path substring.
    #[arg(long)]
    pub path: Option<String>,
    /// Display as nested tree instead of flat list.
    #[arg(long)]
    pub tree: bool,
}
