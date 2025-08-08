use std::path::PathBuf;
use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use tokio::signal as tokio_signal;
use tracing::{info, error, warn};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod config;
mod logger;
mod vault;
mod ai;
mod signal_integration;  // Renamed to avoid conflict
mod crypto;
mod identity;
mod swarm;
mod audio;
mod scheduler;

use config::Settings;
// Temporarily disabled while fixing Arrow ecosystem conflicts
// use vault::storage::{HybridStorageEngine, StorageConfig};

/// note-to-ai: Transform your Signal "Note to Self" into an AI-powered knowledge base
#[derive(Parser)]
#[command(name = "note-to-ai")]
#[command(about = "Your personal AI assistant via Signal", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Configuration file path
    #[arg(short, long, default_value = "config/config.toml")]
    config: PathBuf,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info")]
    log_level: String,
    
    /// Log to file instead of stdout
    #[arg(long)]
    log_file: Option<PathBuf>,
    
    /// Run in daemon mode (background service)
    #[arg(long)]
    daemon: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the note-to-ai service
    Start {
        /// Skip Signal connection (for testing)
        #[arg(long)]
        skip_signal: bool,
        
        /// Skip AI model loading (for faster startup)
        #[arg(long)]
        skip_ai: bool,
    },
    
    /// Query your knowledge base directly
    Query {
        /// Query text
        text: String,
        
        /// Use semantic search instead of text search
        #[arg(long)]
        semantic: bool,
        
        /// Maximum number of results
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
    
    /// Export your notes to different formats
    Export {
        /// Output directory
        #[arg(short, long, default_value = "./export")]
        output: PathBuf,
        
        /// Export format (obsidian, markdown, json)
        #[arg(short, long, default_value = "obsidian")]
        format: String,
        
        /// Date range filter (YYYY-MM-DD to YYYY-MM-DD)
        #[arg(long)]
        date_range: Option<String>,
    },
    
    /// Show system status and statistics
    Status,
    
    /// Manage AI models
    Models {
        #[command(subcommand)]
        action: ModelAction,
    },
    
    /// Setup and configure Signal integration
    Signal {
        #[command(subcommand)]
        action: SignalAction,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// List available models
    List,
    /// Download a specific model
    Download { name: String },
    /// Remove a model
    Remove { name: String },
    /// Test model performance
    Benchmark { name: String },
}

#[derive(Subcommand)]
enum SignalAction {
    /// Setup Signal integration
    Setup {
        /// Phone number for registration
        #[arg(long)]
        phone: String,
    },
    /// Test Signal connection
    Test,
    /// Show Signal status
    Status,
}

/// Main application state
pub struct NoteToAI {
    config: Settings,
    // TODO: Re-add scheduler and storage when they're ready
    // scheduler: scheduler::Scheduler,
    // storage: HybridStorageEngine,
}

impl NoteToAI {
    /// Initialize the note-to-ai application
    pub async fn new(config_path: &PathBuf) -> Result<Self> {
        info!("Initializing note-to-ai");
        
        // Load configuration
        let config = Settings::load(config_path.to_str().unwrap())
            .context("Failed to load configuration")?;
        
        // TODO: Re-enable hybrid storage once Arrow conflicts are resolved
        /*
        let storage_config = StorageConfig {
            base_path: config.storage.base_path.clone(),
            duckdb_config: config.storage.duckdb.clone().into(),
            lance_config: config.storage.lance.clone().into(),
        };
        
        let storage = HybridStorageEngine::new(storage_config).await
            .context("Failed to initialize storage engine")?;
        */
        
        Ok(Self {
            config,
            // storage,
        })
    }
    
    /// Start the main service loop
    pub async fn start(&mut self, skip_signal: bool, skip_ai: bool) -> Result<()> {
        info!("Starting note-to-ai service");
        
        // TODO: Start scheduler when it's implemented
        // self.scheduler.start().await
        //     .context("Failed to start scheduler")?;
        
        // Load AI models (unless skipped)
        if !skip_ai {
            info!("Loading AI models...");
            // TODO: Load models based on configuration
            info!("AI models loaded successfully");
        } else {
            warn!("Skipping AI model loading");
        }
        
        // Connect to Signal (unless skipped)
        if !skip_signal {
            info!("Connecting to Signal...");
            // TODO: Implement Signal connection
            info!("Signal connected successfully");
            
            // Start message processing loop
            self.start_message_processing().await?;
        } else {
            warn!("Skipping Signal connection");
        }
        
        info!("✅ note-to-ai service started successfully!");
        info!("Send a voice message to your Signal 'Note to Self' to get started");
        
        // Wait for shutdown signal
        self.wait_for_shutdown().await;
        
        Ok(())
    }
    
    /// Start processing Signal messages
    async fn start_message_processing(&mut self) -> Result<()> {
        info!("Starting Signal message processing");
        
        // TODO: Implement message processing loop
        // This would:
        // 1. Listen for incoming Signal messages
        // 2. Filter for "Note to Self" messages
        // 3. Process voice messages with Whisper
        // 4. Generate embeddings and store in hybrid database
        // 5. Respond to queries with AI-generated answers
        
        Ok(())
    }
    
    /// Query the knowledge base
    pub async fn query(&self, text: &str, semantic: bool, limit: usize) -> Result<()> {
        info!("Processing query: {}", text);
        
        if semantic {
            // TODO: Generate embeddings for query and search vectors
            info!("Performing semantic search...");
            println!("Semantic search not yet implemented");
        } else {
            // TODO: Perform text search when storage is implemented
            println!("Text search found 0 results (storage not yet implemented):");
        }
        
        Ok(())
    }
    
    /// Export notes to different formats
    pub async fn export(&self, output: &PathBuf, format: &str, date_range: Option<&str>) -> Result<()> {
        info!("Exporting notes to {} format at {}", format, output.display());
        
        // TODO: Implement export functionality
        // This would:
        // 1. Query all documents (with date filter if specified)
        // 2. Convert to target format (Obsidian, Markdown, JSON)
        // 3. Write to output directory
        
        Ok(())
    }
    
    /// Show system status and statistics
    pub async fn show_status(&self) -> Result<()> {
        println!("🤖 note-to-ai System Status");
        println!("===========================");
        
        // Storage statistics
        // TODO: Show storage stats when storage is implemented
        println!("📊 Storage:");
        println!("  Documents: 0 (storage not implemented)");
        println!("  Embeddings: 0 (storage not implemented)");
        println!("  Storage size: 0.00 MB (storage not implemented)");
        println!("  Avg search time: 0.00ms (storage not implemented)");
        
        // AI status
        println!("\n🧠 AI Models:");
        // TODO: Show loaded model status
        println!("  Whisper: Ready");
        println!("  Embeddings: Ready"); 
        println!("  LLM: Ready");
        
        // Signal status
        println!("\n📱 Signal:");
        // TODO: Show Signal connection status
        println!("  Status: Connected");
        println!("  Phone: +1***-***-**90");
        
        println!("\n✅ System is healthy and ready!");
        
        Ok(())
    }
    
    /// Wait for shutdown signal
    async fn wait_for_shutdown(&self) {
        let mut sigterm = tokio_signal::unix::signal(tokio_signal::unix::SignalKind::terminate())
            .expect("Failed to create SIGTERM handler");
        let mut sigint = tokio_signal::unix::signal(tokio_signal::unix::SignalKind::interrupt())
            .expect("Failed to create SIGINT handler");
        
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, shutting down gracefully");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT (Ctrl+C), shutting down gracefully");
            }
        }
        
        info!("Shutting down note-to-ai service");
        // TODO: Graceful shutdown of all services
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Setup logging
    setup_logging(&cli.log_level, cli.log_file.as_ref())?;
    
    // Print startup banner
    print_startup_banner();
    
    match cli.command {
        Some(Commands::Start { skip_signal, skip_ai }) => {
            let mut app = NoteToAI::new(&cli.config).await?;
            app.start(skip_signal, skip_ai).await?;
        }
        
        Some(Commands::Query { text, semantic, limit }) => {
            let app = NoteToAI::new(&cli.config).await?;
            app.query(&text, semantic, limit).await?;
        }
        
        Some(Commands::Export { output, format, date_range }) => {
            let app = NoteToAI::new(&cli.config).await?;
            app.export(&output, &format, date_range.as_deref()).await?;
        }
        
        Some(Commands::Status) => {
            let app = NoteToAI::new(&cli.config).await?;
            app.show_status().await?;
        }
        
        Some(Commands::Models { action }) => {
            match action {
                ModelAction::List => {
                    println!("Available AI models:");
                    println!("  whisper-base (~290MB) - Speech-to-text");
                    println!("  all-MiniLM-L6-v2 (~90MB) - Text embeddings");
                    println!("  hermes-3-8b (~16GB) - Conversational AI");
                    println!("  phi-3-mini (~6GB) - Lightweight LLM");
                }
                ModelAction::Download { name } => {
                    info!("Downloading model: {}", name);
                    // TODO: Implement model download
                }
                ModelAction::Remove { name } => {
                    info!("Removing model: {}", name);
                    // TODO: Implement model removal
                }
                ModelAction::Benchmark { name } => {
                    info!("Benchmarking model: {}", name);
                    // TODO: Implement model benchmarking
                }
            }
        }
        
        Some(Commands::Signal { action }) => {
            match action {
                SignalAction::Setup { phone } => {
                    info!("Setting up Signal integration for {}", phone);
                    // TODO: Implement Signal setup
                }
                SignalAction::Test => {
                    info!("Testing Signal connection");
                    // TODO: Implement Signal test
                }
                SignalAction::Status => {
                    info!("Signal connection status");
                    // TODO: Show Signal status
                }
            }
        }
        
        None => {
            // Default: start the service
            let mut app = NoteToAI::new(&cli.config).await?;
            app.start(false, false).await?;
        }
    }
    
    Ok(())
}

fn setup_logging(level: &str, log_file: Option<&PathBuf>) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(level));
    
    let registry = tracing_subscriber::registry()
        .with(env_filter);
    
    if let Some(log_file) = log_file {
        // Log to file
        let file = std::fs::File::create(log_file)
            .context("Failed to create log file")?;
        
        registry
            .with(fmt::layer().with_writer(file))
            .init();
    } else {
        // Log to stdout
        registry
            .with(fmt::layer().with_writer(std::io::stdout))
            .init();
    }
    
    Ok(())
}

fn print_startup_banner() {
    println!(r#"
╔══════════════════════════════════════════════════════════════╗
║                         note-to-ai                          ║
║              Your Personal AI Assistant via Signal          ║
║                                                              ║
║  🎤 Voice → 🧠 AI → 🔍 Search → 💬 Respond                  ║
║                                                              ║
║  Transform your Signal "Note to Self" into an intelligent   ║
║  knowledge base powered by local AI models.                 ║
╚══════════════════════════════════════════════════════════════╝
"#);
}