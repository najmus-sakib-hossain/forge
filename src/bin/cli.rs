use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use dx_forge::{context, server, storage, watcher_legacy};
use tracing_subscriber::{EnvFilter};
use tracing_subscriber::prelude::*;
use tracing_appender::rolling;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "forge")]
#[command(
    about = "Next-generation version control with operation-level tracking, CRDT-based sync, and seamless Git integration",
    version
)]
#[command(after_help = "Forge Features:
- Operation-level version control with CRDT for conflict-free collaboration
- Real-time sync between multiple peers via WebSocket
- Character-level anchors and permalinks for precise code references
- AI-powered code annotations and context exploration
- Git repository synchronization and integration
- Collaborative server for multi-user editing
- Time-travel debugging to view file states at any timestamp
- Comprehensive operation logging and querying
- Seamless Git command support without 'git' prefix

All Git commands are supported without the 'git' prefix. Use 'forge <git-command>' instead of 'git <git-command>'.

Main Porcelain Commands:
   add, am, archive, backfill, bisect, branch, bundle, checkout, cherry-pick, citool, clean, clone, commit, describe, diff, fetch, format-patch, gc, gitk, grep, gui, init, log, maintenance, merge, mv, notes, pull, push, range-diff, rebase, reset, restore, revert, rm, scalar, shortlog, show, sparse-checkout, stash, status, submodule, survey, switch, tag, worktree

Ancillary Commands / Manipulators:
   config, fast-export, fast-import, filter-branch, mergetool, pack-refs, prune, reflog, refs, remote, repack, replace

Ancillary Commands / Interrogators:
   annotate, blame, bugreport, count-objects, diagnose, difftool, fsck, gitweb, help, instaweb, merge-tree, rerere, show-branch, verify-commit, verify-tag, version, whatchanged

Interacting with Others:
   archimport, cvsexportcommit, cvsimport, cvsserver, imap-send, p4, quiltimport, request-pull, send-email, svn

Low-level Commands / Manipulators:
   apply, checkout-index, commit-graph, commit-tree, hash-object, index-pack, merge-file, merge-index, mktag, mktree, multi-pack-index, pack-objects, prune-packed, read-tree, replay, symbolic-ref, unpack-objects, update-index, update-ref, write-tree

Low-level Commands / Interrogators:
   cat-file, cherry, diff-files, diff-index, diff-pairs, diff-tree, for-each-ref, for-each-repo, get-tar-commit-id, ls-files, ls-remote, ls-tree, merge-base, name-rev, pack-redundant, rev-list, rev-parse, show-index, show-ref, unpack-file, var, verify-pack

Low-level Commands / Syncing Repositories:
   daemon, fetch-pack, http-backend, send-pack, update-server-info

Low-level Commands / Internal Helpers:
   check-attr, check-ignore, check-mailmap, check-ref-format, column, credential, credential-cache, credential-store, fmt-merge-msg, hook, interpret-trailers, mailinfo, mailsplit, merge-one-file, patch-id, sh-i18n, sh-setup, stripspace

External commands:
   askpass, askyesno, credential-helper-selector, credential-manager, flow, lfs, update-git-for-windows")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Forge repository
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Watch for changes and track operations
    Watch {
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Enable real-time sync
        #[arg(long)]
        sync: bool,

        /// WebSocket peer(s) to connect, e.g. ws://localhost:3000/ws
        #[arg(long, value_name = "URL")]
        peer: Vec<String>,
    },

    /// Query the operation log
    OpLog {
        #[arg(short, long)]
        file: Option<PathBuf>,

        #[arg(short, long)]
        limit: Option<usize>,
    },

    /// Create a character-level anchor/permalink
    Anchor {
        file: PathBuf,
        line: usize,
        column: usize,

        #[arg(short, long)]
        message: Option<String>,
    },

    /// Annotate code with context
    Annotate {
        file: PathBuf,
        line: usize,

        #[arg(short, long)]
        message: String,

        #[arg(long)]
        ai: bool,
    },

    /// Show annotations and context for a file
    Context {
        file: PathBuf,

        #[arg(short, long)]
        line: Option<usize>,
    },

    /// Sync Forge repository
    ForgeSync {
        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Any unrecognized subcommand will be passed to the system `git`.
    #[command(external_subcommand)]
    GitPassthrough(Vec<String>),

    /// Start collaborative server
    Serve {
        #[arg(short, long, default_value = "3000")]
        port: u16,

        #[arg(default_value = ".")]
        path: PathBuf,
    },

    /// Show time-travel view of a file
    TimeTravel {
        file: PathBuf,

        #[arg(short, long)]
        timestamp: Option<String>,
    },

    /// Update DX-managed components
    Update {
        /// Component to update (or "all")
        component: Option<String>,

        /// Force update (skip conflict checks)
        #[arg(long)]
        force: bool,
    },

    /// List managed DX components
    Components {
        /// Show component details
        #[arg(long)]
        verbose: bool,
    },

    /// Register a component for tracking
    Register {
        /// Path to component file
        path: PathBuf,

        /// Component source (e.g., "dx-ui")
        #[arg(long)]
        source: String,

        /// Component name (e.g., "Button")
        #[arg(long)]
        name: String,

        /// Component version
        #[arg(long)]
        version: String,
    },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    // Initialize structured logging to logs/forge.log with daily rotation
    let log_dir = std::env::current_dir()?.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    // Use daily rotation to prevent unbounded log file growth
    let file_appender = rolling::daily(&log_dir, "forge.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false);

    let stdout_layer = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);

    let env_filter = match EnvFilter::try_from_default_env() {
        Ok(f) => f,
        Err(_) => EnvFilter::new("warn"),  // Changed from 'info' to 'warn' to reduce log spam
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    // keep the guard alive for the lifetime of the program
    let _guard = guard;

    // Only log at startup if debug/trace is enabled
    // info!("Logging initialized: {}", log_dir.join("forge.log").display());

    let cli = Cli::parse();

    let command = match cli.command {
        Some(cmd) => cmd,
        None => Commands::Watch {
            path: ".".into(),
            sync: false,
            peer: vec![],
        },
    };

    match command {
        Commands::Init { path } => {
            println!(
                "{}",
                "🚀 Initializing Forge DeltaDB repository...".cyan().bold()
            );
            storage::init(&path).await?;
            println!("{}", "✓ Repository initialized successfully!".green());
            println!("\n{}", "Next steps:".yellow());
            println!(
                "  1. {} - Start tracking operations",
                "forge watch".bright_white()
            );
            println!("  2. {} - View operation log", "forge oplog".bright_white());
            println!(
                "  3. {} - Add context to code",
                "forge annotate <file> <line> -m \"message\"".bright_white()
            );
        }

        Commands::Watch { path, sync, peer } => {
            println!("{}", "✔ Starting operation-level tracking...".cyan().bold());
            watcher_legacy::watch(path, sync, peer).await?;
        }

        Commands::OpLog { file, limit } => {
            storage::show_log(file, limit.unwrap_or(50)).await?;
        }

        Commands::Anchor {
            file,
            line,
            column,
            message,
        } => {
            let anchor = context::create_anchor(&file, line, column, message).await?;
            println!(
                "{} Created anchor: {}",
                "✓".green(),
                anchor.id.to_string().bright_yellow()
            );
            println!("  Permalink: {}", anchor.permalink().bright_blue());
        }

        Commands::Annotate {
            file,
            line,
            message,
            ai,
        } => {
            context::annotate(&file, line, &message, ai).await?;
            println!("{} Annotation added", "✓".green());
        }

        Commands::Context { file, line } => {
            context::show_context(&file, line).await?;
        }

        Commands::ForgeSync { path } => {
            storage::git_sync(&path).await?;
        }

        Commands::GitPassthrough(args) => {
            use tokio::process::Command;
            let status = if args.is_empty() {
                Command::new("git").status().await?
            } else {
                Command::new("git").args(args).status().await?
            };
            if !status.success() {
                eprintln!("git exited with status: {}", status);
            }
        }

        Commands::Serve { port, path } => {
            println!(
                "{}",
                format!("🌐 Starting server on port {}...", port)
                    .cyan()
                    .bold()
            );
            server::start(port, path).await?;
        }

        Commands::TimeTravel { file, timestamp } => {
            storage::time_travel(&file, timestamp).await?;
        }

        Commands::Update {
            component,
            force: _,
        } => {
            use dx_forge::context::ComponentStateManager;

            let forge_dir = std::env::current_dir()?.join(".dx/forge");
            let state_mgr = ComponentStateManager::new(&forge_dir)?;

            if let Some(comp_name) = component {
                if comp_name == "all" {
                    println!("{}", "🔄 Updating all components...".cyan().bold());
                    let components = state_mgr.list_components();
                    for comp in &components {
                        println!(
                            "\n{} Checking {}...",
                            "→".bright_black(),
                            comp.name.bright_cyan()
                        );
                        // In production, fetch remote version and apply update
                        println!(
                            "   {} (placeholder - would fetch and update)",
                            "→".bright_black()
                        );
                    }
                } else {
                    println!(
                        "{}",
                        format!("🔄 Updating component: {}...", comp_name)
                            .cyan()
                            .bold()
                    );
                    // In production, fetch remote version and apply update
                    println!(
                        "   {} (placeholder - would fetch and update)",
                        "→".bright_black()
                    );
                }
            } else {
                println!("{}", "Please specify a component name or 'all'".yellow());
            }
        }

        Commands::Components { verbose } => {
            use dx_forge::context::ComponentStateManager;

            let forge_dir = std::env::current_dir()?.join(".dx/forge");
            let state_mgr = ComponentStateManager::new(&forge_dir)?;

            let components = state_mgr.list_components();

            if components.is_empty() {
                println!("{}", "No managed components found.".yellow());
                println!("\n{}", "To register a component:".bright_black());
                println!(
                    "  {}",
                    "forge register <path> --source dx-ui --name Button --version 1.0.0"
                        .bright_white()
                );
            } else {
                println!("{}", "📦 Managed Components".cyan().bold());
                println!("{}", "═".repeat(80).bright_black());

                for comp in &components {
                    println!(
                        "\n{} {} {}",
                        "●".bright_green(),
                        comp.name.bright_cyan().bold(),
                        format!("v{}", comp.version).bright_black()
                    );
                    println!("   {} {}", "Source:".bright_black(), comp.source);
                    println!("   {} {}", "Path:  ".bright_black(), comp.path);

                    if verbose {
                        println!("   {} {}", "Hash:  ".bright_black(), &comp.base_hash[..16]);
                        println!(
                            "   {} {}",
                            "Added: ".bright_black(),
                            comp.installed_at.format("%Y-%m-%d %H:%M:%S")
                        );
                    }
                }

                println!("\n{}", "─".repeat(80).bright_black());
                println!(
                    "{} {} | {} {}",
                    format!("{} components", components.len())
                        .bright_white()
                        .bold(),
                    "Use --verbose for details".bright_black(),
                    "forge update <name>".bright_white(),
                    "to update".bright_black()
                );
            }
        }

        Commands::Register {
            path,
            source,
            name,
            version,
        } => {
            use dx_forge::context::ComponentStateManager;

            let forge_dir = std::env::current_dir()?.join(".dx/forge");
            let mut state_mgr = ComponentStateManager::new(&forge_dir)?;

            // Read component content
            let content = tokio::fs::read_to_string(&path).await?;

            // Register component
            state_mgr.register_component(&path, &source, &name, &version, &content)?;

            println!(
                "{} Registered {} {} {}",
                "✓".green(),
                name.bright_cyan().bold(),
                format!("v{}", version).bright_black(),
                format!("from {}", source).bright_black()
            );
            println!("   {} {}", "Path:".bright_black(), path.display());
        }
    }

    Ok(())
}
