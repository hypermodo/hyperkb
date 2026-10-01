use clap::{Parser, Subcommand};
use hyperkb_rs::core::Scanner;
use hyperkb_rs::domain::BrowseOptions;
use hyperkb_rs::storage::{Database, Queries};
use hyperkb_rs::transport::McpServer;
use hyperkb_rs::ui;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "hyperkb")]
#[command(about = "Fast, resource-efficient local developer knowledge & proactive risk engine")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value = ".")]
    root: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Index Markdown files in the target directory
    Index {
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
    /// Search indexed knowledge and private memory
    Search {
        query: String,
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
        #[arg(long)]
        include_private: bool,
    },
    /// Browse indexed documents by category or project
    Browse {
        #[arg(short, long, default_value = "all")]
        category: String,
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long)]
        topic: Option<String>,
    },
    /// Check planned files against cited risks and architectural boundaries
    #[command(alias = "check_work", alias = "check")]
    CheckWork {
        /// Files to check
        #[arg(required = true)]
        files: Vec<String>,
        /// Target version label (e.g. v2.0)
        #[arg(short, long)]
        version: Option<String>,
        /// Target deployment environment (e.g. production, staging)
        #[arg(short, long)]
        env: Option<String>,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Run as Model Context Protocol (MCP) stdio server for AI agents
    Mcp,
    /// Remember a private local note
    Remember {
        title: String,
        content: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let db_path = cli.root.join(".hyperkb").join("hyperkb.db");
    let db = Database::open_or_create(&db_path, "local_collection", "default_profile")?;

    match cli.command {
        Some(Commands::Index { dir }) => {
            println!("Indexing Markdown documents in: {}", dir.display());
            let report = Scanner::index_directory(db.conn(), &dir, "local_collection")?;
            println!(
                "Indexed {} documents ({} added, {} updated, {} unchanged, {} errors)",
                report.scanned, report.added, report.updated, report.unchanged, report.errors
            );
        }
        Some(Commands::Search {
            query,
            limit,
            include_private,
        }) => {
            let hits = Queries::search(
                db.conn(),
                &["local_collection".into()],
                "default_profile",
                &query,
                limit,
                include_private,
            )?;
            println!("Found {} results for '{}':", hits.len(), query);
            for hit in hits {
                println!(
                    "- [{}] {} ({})\n  {}",
                    hit.status.as_str(),
                    hit.title,
                    hit.source,
                    hit.snippet
                );
            }
        }
        Some(Commands::Browse {
            category,
            project,
            topic,
        }) => {
            let opts = BrowseOptions {
                category,
                project,
                topic,
                ..Default::default()
            };
            let (docs, total) = Queries::browse(db.conn(), &["local_collection".into()], &opts)?;
            println!("Browse ({} total documents):", total);
            for doc in docs {
                println!("- [{}] {} ({})", doc.status.as_str(), doc.title, doc.path);
            }
        }
        Some(Commands::CheckWork {
            files,
            version,
            env,
            json,
        }) => {
            let check = Queries::check_work(
                db.conn(),
                "local_collection",
                &files,
                version.as_deref(),
                env.as_deref(),
            )?;

            if json {
                println!("{}", serde_json::to_string_pretty(&check)?);
            } else {
                println!("Checked {} path(s):", check.checked_paths.len());
                for path in &check.checked_paths {
                    println!("  - {}", path);
                }
                println!();

                if check.matches.is_empty() {
                    println!("✓ Clean: No cited open risks or path hazards detected.");
                } else {
                    println!("⚠️  Found {} cited risk(s):", check.matches.len());
                    for m in &check.matches {
                        let status_tag = match m.applicability {
                            hyperkb_rs::domain::RiskApplicability::Applies => "[APPLIES]",
                            hyperkb_rs::domain::RiskApplicability::Unknown => "[UNKNOWN]",
                            hyperkb_rs::domain::RiskApplicability::NotApplicable => "[N/A]",
                        };
                        println!(
                            "  {} {} ({})",
                            status_tag, m.document.title, m.document.path
                        );
                        println!("     Matched: {}", m.matched_paths.join(", "));
                        println!("     Reason:  {}", m.reason);
                        if let Some(ref ack) = m.acknowledgement {
                            println!("     Acknowledged: {}", ack);
                        } else if m.acknowledged {
                            println!("     Acknowledged: yes");
                        }
                    }

                    if check.has_open_risks() {
                        println!("\n❌ Blocked: Unacknowledged risks apply to your planned changes.");
                        std::process::exit(1);
                    }
                }
            }
        }
        Some(Commands::Mcp) => {
            let docs_dir = PathBuf::from("../hyperkb/docs");
            if docs_dir.exists() {
                let _ = Scanner::index_directory(db.conn(), &docs_dir, "local_collection");
            } else {
                let _ = Scanner::index_directory(db.conn(), &cli.root, "local_collection");
            }
            McpServer::run_stdio(db.conn(), "local_collection", "default_profile")?;
        }
        Some(Commands::Remember { title, content }) => {
            let id = uuid::Uuid::now_v7().to_string();
            Queries::remember(
                db.conn(),
                &id,
                "default_profile",
                &title,
                &content,
                "note",
            )?;
            println!("Saved private note: {} ({})", title, id);
        }
        None => {
            // Index existing docs in workspace so the TUI opens with real knowledge ready to browse
            let docs_dir = PathBuf::from("../hyperkb/docs");
            if docs_dir.exists() {
                let _ = Scanner::index_directory(db.conn(), &docs_dir, "local_collection");
            } else {
                let _ = Scanner::index_directory(db.conn(), &cli.root, "local_collection");
            }

            // Launch the full-screen Ratatui TUI
            ui::run(&db)?;
        }
    }

    Ok(())
}
