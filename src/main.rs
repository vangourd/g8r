use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;

mod api;
mod cli;
mod controller;
mod db;
mod github;
mod modules;
mod nickel;
mod queue;
mod secrets;
mod stack;
mod telemetry;
mod utils;

use api::ApiServer;
use cli::{Cli, Commands};
use db::StateManager;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    
    if let Err(e) = telemetry::init_telemetry() {
        eprintln!("Failed to initialize telemetry: {}", e);
    }

    let cli = Cli::parse();

    let database_url = cli.database_url.clone()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .context("DATABASE_URL must be set")?;

    let state = StateManager::new(&database_url).await
        .context("Failed to connect to database")?;

    match &cli.command {
        Commands::Serve { host, port } => {
            serve_command(state, host.clone(), *port).await?;
        }
    }

    Ok(())
}


async fn serve_command(state: StateManager, host: String, port: u16) -> Result<()> {
    info!("Starting G8R API server");
    
    let server = ApiServer::new(state, host, port);
    server.run().await?;
    
    Ok(())
}
