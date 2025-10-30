use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "g8r")]
#[command(about = "Infrastructure automation platform", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(long)]
    pub database_url: Option<String>,

    #[arg(long, default_value = "us-east-2")]
    pub aws_region: String,

    #[arg(long)]
    pub github_token: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    Serve {
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        #[arg(long, default_value = "8080")]
        port: u16,
    },
}
