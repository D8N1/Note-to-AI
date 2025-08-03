use clap::Parser;
use note_to_ai::{config::Settings, logger, Result};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "note-to-ai")]
#[command(about = "Local-first Obsidian vault manager with quantum-resistant crypto")]
struct Cli {
    #[arg(short, long, default_value = "config/config.toml")]
    config: String,
    
    #[arg(short, long)]
    vault_path: Option<String>,
    
    #[arg(long)]
    daemon: bool,
    
    #[arg(long)]
    setup: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if cli.setup {
        return setup_application().await;
    }
    
    // Load configuration
    let settings = Settings::load(&cli.config)?;
    
    // Initialize logging
    logger::init(&settings.logging)?;
    
    info!("Starting note-to-ai v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration loaded from: {}", cli.config);
    
    if cli.daemon {
        run_daemon(settings).await?;
    } else {
        run_interactive(settings, cli.vault_path).await?;
    }
    
    Ok(())
}

async fn setup_application() -> Result<()> {
    println!("Setting up note-to-ai...");
    // TODO: Implement setup wizard
    Ok(())
}

async fn run_daemon(settings: Settings) -> Result<()> {
    info!("Running in daemon mode");
    // TODO: Implement daemon mode with all services
    Ok(())
}

async fn run_interactive(settings: Settings, vault_path: Option<String>) -> Result<()> {
    info!("Running in interactive mode");
    // TODO: Implement interactive mode
    Ok(())
} 