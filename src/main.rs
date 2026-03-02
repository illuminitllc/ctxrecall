#![allow(dead_code, unused_imports)]

mod action;
mod app;
mod components;
mod db;
mod errors;
mod event;
mod logging;
mod tmux;
mod tracker;
mod tui;

// Stub modules for later phases
mod claude;
mod config;
mod llm;
mod widgets;

use clap::Parser;
use directories::ProjectDirs;

use crate::app::App;
use crate::tmux::TmuxManager;

#[derive(Parser, Debug)]
#[command(name = "ctxrec", about = "TUI issue manager with Claude Code integration")]
struct Cli {
    /// Run directly in TUI mode (used internally when launched inside tmux)
    #[arg(long)]
    in_tmux: bool,

    /// Skip tmux bootstrap and run TUI directly
    #[arg(long)]
    no_tmux: bool,

    /// Linear API key (can also be set in-app)
    #[arg(long, env = "LINEAR_API_KEY")]
    linear_api_key: Option<String>,

    /// Set working directory for a project: --set-project-dir "ProjectName=/path/to/repo"
    #[arg(long)]
    set_project_dir: Option<String>,

    /// Set working directory for a team: --set-team-dir "TeamName=/path/to/repo"
    #[arg(long)]
    set_team_dir: Option<String>,

    /// Set LLM provider for transcript summarization: claude, openai, or ollama
    #[arg(long)]
    set_llm_provider: Option<String>,

    /// Set API key for the configured LLM provider
    #[arg(long)]
    set_llm_api_key: Option<String>,

    /// Set model name override for the LLM provider
    #[arg(long)]
    set_llm_model: Option<String>,

    /// Set Ollama base URL (default: http://localhost:11434)
    #[arg(long)]
    set_llm_ollama_url: Option<String>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let dirs =
        ProjectDirs::from("", "", "ctxrecall").expect("Failed to determine project directories");

    let data_dir = dirs.data_dir().to_path_buf();
    let log_dir = data_dir.join("logs");

    // Initialize logging (keep guard alive for the duration of the program)
    let _log_guard = logging::init_logging(&log_dir);

    tracing::info!("ctxrecall starting");

    // Initialize database
    let db_path = data_dir.join("ctxrecall.db");
    let conn = db::init_db(&db_path)?;

    // Store API key if provided via CLI
    if let Some(api_key) = &cli.linear_api_key {
        db::config_repo::set_config(&conn, "linear_api_key", api_key)?;
        tracing::info!("Linear API key stored from CLI argument");
    }

    // Set project/team directory mappings
    if let Some(mapping) = &cli.set_project_dir {
        if let Some((name, dir)) = mapping.split_once('=') {
            db::config_repo::set_config(&conn, &format!("project_dir:{name}"), dir)?;
            println!("Project '{name}' directory set to: {dir}");
            return Ok(());
        } else {
            eprintln!("Invalid format. Use: --set-project-dir \"ProjectName=/path/to/repo\"");
            std::process::exit(1);
        }
    }
    if let Some(mapping) = &cli.set_team_dir {
        if let Some((name, dir)) = mapping.split_once('=') {
            db::config_repo::set_config(&conn, &format!("team_dir:{name}"), dir)?;
            println!("Team '{name}' directory set to: {dir}");
            return Ok(());
        } else {
            eprintln!("Invalid format. Use: --set-team-dir \"TeamName=/path/to/repo\"");
            std::process::exit(1);
        }
    }

    // LLM configuration flags
    if let Some(provider) = &cli.set_llm_provider {
        match provider.as_str() {
            "claude" | "openai" | "ollama" => {
                db::config_repo::set_config(&conn, "llm_provider", provider)?;
                println!("LLM provider set to: {provider}");
            }
            _ => {
                eprintln!("Unknown provider '{provider}'. Use: claude, openai, or ollama");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    if let Some(key) = &cli.set_llm_api_key {
        db::config_repo::set_config(&conn, "llm_api_key", key)?;
        println!("LLM API key stored");
        return Ok(());
    }
    if let Some(model) = &cli.set_llm_model {
        db::config_repo::set_config(&conn, "llm_model", model)?;
        println!("LLM model set to: {model}");
        return Ok(());
    }
    if let Some(url) = &cli.set_llm_ollama_url {
        db::config_repo::set_config(&conn, "llm_ollama_url", url)?;
        println!("Ollama URL set to: {url}");
        return Ok(());
    }

    // Tmux bootstrap logic
    if !cli.in_tmux && !cli.no_tmux {
        if !TmuxManager::is_inside_tmux() {
            tracing::info!("Not inside tmux, bootstrapping session");
            TmuxManager::bootstrap()?;
            return Ok(());
        }
    }

    // Resolve API key: CLI arg > DB stored > env
    let api_key = cli
        .linear_api_key
        .or_else(|| db::config_repo::get_config(&conn, "linear_api_key").ok().flatten());

    // Create and run app
    let mut app = App::new(conn, data_dir);

    if let Some(key) = api_key {
        tracing::info!("Starting Linear sync");
        app.start_sync(key);
    } else {
        tracing::warn!("No Linear API key configured. Use --linear-api-key or LINEAR_API_KEY env var.");
    }

    tracing::info!("Starting TUI");
    app.run().await?;

    tracing::info!("ctxrecall shutting down");
    Ok(())
}
