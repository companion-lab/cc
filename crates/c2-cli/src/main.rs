use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{EnvFilter, fmt};

// ── CLI definition ─────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "c2",
    version = env!("CARGO_PKG_VERSION"),
    about = "Companion CLI — agentic coding assistant",
    long_about = None,
)]
struct Cli {
    /// Working directory (default: current directory)
    #[arg(short = 'C', long, global = true)]
    cwd: Option<PathBuf>,

    /// Verbose logging (pass twice for debug)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a one-shot prompt and print the response (non-interactive)
    Run {
        /// The prompt to send
        prompt: String,

        /// Continue an existing session by ID
        #[arg(short, long)]
        session: Option<String>,

        /// Model override (e.g. claude-3-7-sonnet-20250219)
        #[arg(short, long)]
        model: Option<String>,
    },

    /// Start the daemon server (HTTP + SSE)
    Serve {
        /// Address to listen on
        #[arg(short, long, default_value = "127.0.0.1:7773")]
        addr: String,
    },

    /// Manage sessions
    Session {
        #[command(subcommand)]
        cmd: SessionCmd,
    },

    /// List configured providers and models
    Providers,

    /// Check for updates and upgrade c2 to the latest version
    Update,

    /// Print shell completion script
    Completion {
        /// Shell to generate for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum SessionCmd {
    /// List all sessions
    List,
    /// Delete a session
    Delete {
        /// Session ID
        id: String,
    },
}

// ── Entry point ────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialise tracing
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();

    // Resolve working directory
    let cwd = match cli.cwd {
        Some(d) => d,
        None => std::env::current_dir().context("cannot determine current directory")?,
    };

    match cli.command {
        // Default: open TUI
        None => {
            c2_tui::run().await?;
        }

        Some(Commands::Run { prompt, session, model }) => {
            cmd_run(cwd, prompt, session, model).await?;
        }

        Some(Commands::Serve { addr }) => {
            cmd_serve(cwd, addr).await?;
        }

        Some(Commands::Session { cmd }) => {
            cmd_session(cwd, cmd).await?;
        }

        Some(Commands::Providers) => {
            cmd_providers(cwd).await?;
        }

        Some(Commands::Update) => {
            cmd_update().await?;
        }

        Some(Commands::Completion { shell }) => {
            use clap::CommandFactory;
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "c2",
                &mut std::io::stdout(),
            );
        }
    }

    Ok(())
}

// ── Shared init helpers ────────────────────────────────────────────────────────

async fn init_db(cwd: &PathBuf) -> Result<Arc<c2_storage::Db>> {
    let data_dir = c2_config::Paths::user_data_dir();
    // Also check for project-local .c2/ dir
    let local_data = cwd.join(".c2").join("data");
    let data_dir = if local_data.exists() { local_data } else { data_dir };
    let db = c2_storage::Db::open(&data_dir).await.context("open database")?;
    Ok(Arc::new(db))
}

// ── c2 run ─────────────────────────────────────────────────────────────────────

async fn cmd_run(cwd: PathBuf, prompt: String, session_id: Option<String>, model_override: Option<String>) -> Result<()> {
    // 1. Load config
    let mut config = c2_config::load(&cwd).await.context("load config")?;
    if let Some(m) = model_override {
        config.model = Some(m);
    }

    // 2. Open database
    let db = init_db(&cwd).await?;

    // 3. Build provider registry
    let registry = c2_provider::ProviderRegistry::from_config(&config)
        .await
        .context("init provider")?;
    let model = registry.model();

    // 4. Resolve or create session
    let session = match &session_id {
        Some(id) => {
            c2_core::session::Session::get(&db, id)
                .await?
                .with_context(|| format!("session '{}' not found", id))?
        }
        None => {
            // Create a new session titled with first ~60 chars of prompt
            let title: String = prompt.chars().take(60).collect();
            let sess = c2_core::session::Session::new(
                cwd.to_string_lossy().to_string(),
                title,
            );
            sess.save(&db).await?;
            sess
        }
    };

    tracing::info!("session id: {}", session.id);

    // 5. Create bus (for streaming events)
    let bus = Arc::new(c2_core::bus::Bus::new());

    // 6. Create abort channel
    let (abort_tx, abort_rx) = tokio::sync::watch::channel(false);

    // Handle Ctrl+C
    let abort_tx_ctrlc = abort_tx.clone();
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            let _ = abort_tx_ctrlc.send(true);
        }
    });

    // 7. Spawn event printer — forwards text deltas to stdout
    let mut event_rx = bus.subscribe();
    let printer = tokio::spawn(async move {
        use c2_core::Event;
        use tokio::io::AsyncWriteExt;
        let mut stdout = tokio::io::stdout();
        loop {
            match event_rx.recv().await {
                Ok(Event::TextDelta { delta, .. }) => {
                    let _ = stdout.write_all(delta.as_bytes()).await;
                    let _ = stdout.flush().await;
                }
                Ok(Event::AgentDone { .. }) => break,
                Err(_) => break,
                _ => {}
            }
        }
        // Final newline
        let _ = stdout.write_all(b"\n").await;
    });

    // 8. Run processor
    let processor = c2_agent::processor::Processor::new(model, db, bus);
    processor.run(&session, prompt, abort_rx).await?;

    // 9. Wait for printer to flush
    let _ = printer.await;

    Ok(())
}

// ── c2 serve ──────────────────────────────────────────────────────────────────

async fn cmd_serve(cwd: PathBuf, addr_str: String) -> Result<()> {
    let config = c2_config::load(&cwd).await.context("load config")?;
    let db = init_db(&cwd).await?;
    let bus = Arc::new(c2_core::bus::Bus::new());
    let addr: std::net::SocketAddr = addr_str.parse().context("invalid listen address")?;

    eprintln!("Starting c2 daemon on {addr}");
    c2_server::serve(config, db, bus, addr).await
}

// ── c2 session ────────────────────────────────────────────────────────────────

async fn cmd_session(cwd: PathBuf, cmd: SessionCmd) -> Result<()> {
    let db = init_db(&cwd).await?;

    match cmd {
        SessionCmd::List => {
            let sessions = c2_core::session::Session::list(&db).await?;
            if sessions.is_empty() {
                println!("No sessions.");
                return Ok(());
            }
            println!("{:<26}  {:<20}  {}", "ID", "UPDATED", "TITLE");
            println!("{}", "-".repeat(72));
            for s in sessions {
                use chrono::{DateTime, Utc};
                let updated = DateTime::<Utc>::from_timestamp_millis(s.time_updated)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
                    .unwrap_or_else(|| s.time_updated.to_string());
                println!("{:<26}  {:<20}  {}", s.id, updated, s.title);
            }
        }
        SessionCmd::Delete { id } => {
            c2_core::session::Session::delete(&db, &id).await?;
            println!("Deleted session {id}");
        }
    }
    Ok(())
}

// ── c2 providers ──────────────────────────────────────────────────────────────

async fn cmd_providers(cwd: PathBuf) -> Result<()> {
    let config = c2_config::load(&cwd).await.context("load config")?;

    println!("Configured provider:");
    match &config.provider {
        Some(p) => {
            println!("  id:       {}", p.id);
            if let Some(url) = &p.base_url {
                println!("  base_url: {}", url);
            }
            let has_key = p.api_key.as_deref().map(|k| !k.is_empty()).unwrap_or(false);
            println!("  api_key:  {}", if has_key { "set" } else { "not set" });
        }
        None => println!("  (none — will default to anthropic)"),
    }

    println!("\nActive model: {}", config.model.as_deref().unwrap_or("claude-3-7-sonnet-20250219 (default)"));

    // Try to resolve — confirms credentials work
    match c2_provider::ProviderRegistry::from_config(&config).await {
        Ok(_) => println!("\nProvider credentials: OK"),
        Err(e) => println!("\nProvider credentials: ERROR — {e}"),
    }

    Ok(())
}

// ── c2 update ─────────────────────────────────────────────────────────────────

async fn cmd_update() -> Result<()> {
    println!("Checking for updates...");
    
    // In the future this should hit the GitHub Releases API
    // e.g., https://api.github.com/repos/companion-lab/c2/releases/latest
    // For now we just use a stub since we haven't published binary releases
    
    let current_version = env!("CARGO_PKG_VERSION");
    
    println!("Current version: v{}", current_version);
    println!("Latest version:  v{} (simulated)", current_version);
    println!("\nYou are already running the latest version of c2!");
    
    Ok(())
}
