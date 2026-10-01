use chrono::Utc;
use clap::{Parser, Subcommand};
use hyperkb_rs::core::{
    DecisionWorkflow, Git, GrantStore, MaintenanceManager, RiskWorkflow, Scanner,
};
use hyperkb_rs::domain::{ActionKind, Actor, BrowseOptions, GrantConstraints};
use hyperkb_rs::storage::{Database, Queries};
use hyperkb_rs::transport::McpServer;
use hyperkb_rs::ui;
use std::path::PathBuf;
use uuid::Uuid;

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
        /// Files to check (or use --staged / --changed)
        files: Vec<String>,
        /// Automatically check staged files in Git index
        #[arg(long)]
        staged: bool,
        /// Automatically check uncommitted files in working tree
        #[arg(long)]
        changed: bool,
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
    /// Install a pre-commit risk interception hook into .git/hooks/pre-commit
    InstallHook,
    /// Propose an architectural risk record citing affected file paths
    DraftRisk {
        #[arg(short, long)]
        title: String,
        #[arg(short, long)]
        rationale: String,
        #[arg(short, long, default_value = "Developer")]
        owner: String,
        /// Affected file path patterns (e.g. -p "src/storage/**" -p "auth/*")
        #[arg(short, long, required = true)]
        paths: Vec<String>,
        #[arg(short, long)]
        versions: Vec<String>,
        #[arg(short, long)]
        environments: Vec<String>,
    },
    /// Run as Model Context Protocol (MCP) stdio server for AI agents
    Mcp,
    /// Remember a private local note
    Remember {
        title: String,
        content: String,
    },
    /// Draft a proposed architecture decision document
    #[command(alias = "draft")]
    DraftDecision {
        #[arg(short, long)]
        title: String,
        #[arg(short, long)]
        rationale: String,
        #[arg(short, long, default_value = "Developer")]
        owner: String,
        #[arg(short, long)]
        supersedes: Option<String>,
    },
    /// Review and accept a proposed decision document
    #[command(alias = "accept")]
    AcceptDecision {
        /// Relative path to the proposed decision markdown file
        path: String,
        #[arg(short, long, default_value = "Owner")]
        owner: String,
        #[arg(short, long)]
        supersedes: Option<String>,
    },
    /// Create a verified point-in-time backup snapshot and rotate older snapshots
    Backup {
        #[arg(short, long, default_value_t = 2)]
        keep: usize,
    },
    /// List verified backup snapshots
    ListBackups,
    /// Compact the SQLite database and truncate the WAL journal to reclaim disk space
    Compact,
    /// Manage AuthorityGrants for delegated agent autonomy
    Grant {
        #[command(subcommand)]
        command: GrantCommands,
    },
    /// Acknowledge an open risk with rationale so it no longer blocks check_work
    #[command(alias = "ack")]
    AcknowledgeRisk {
        /// Stable ID or relative path of the risk
        risk_id: String,
        /// Justification or mitigating rationale
        #[arg(short, long)]
        rationale: String,
        /// Responsible human owner
        #[arg(short, long, default_value = "Developer")]
        owner: String,
    },
}

#[derive(Subcommand)]
enum GrantCommands {
    /// Issue a new AuthorityGrant to an agent delegate
    Issue {
        /// Target agent identity (e.g. claude-3-7-sonnet, opencode)
        #[arg(short, long)]
        grantee: String,
        /// Human authorizer / principal (defaults to current user or "Developer")
        #[arg(long)]
        granted_by: Option<String>,
        /// Comma-separated allowed actions (accept-decision, acknowledge-risk, propose-decision, auto-repair)
        #[arg(short, long, default_value = "accept-decision,acknowledge-risk")]
        actions: String,
        /// Comma-separated file path glob patterns (e.g. "docs/decisions/**,projects/**")
        #[arg(short, long, default_value = "*")]
        scope: String,
        /// Maximum allowable diff line count for ratified edits
        #[arg(long, default_value_t = 200)]
        max_diff: usize,
        /// Validity duration in hours (e.g. 24)
        #[arg(long)]
        ttl_hours: Option<u64>,
    },
    /// List active authority grants
    List,
    /// Revoke an existing authority grant
    Revoke {
        /// Grant UUID
        grant_id: String,
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
            mut files,
            staged,
            changed,
            version,
            env,
            json,
        }) => {
            if staged {
                let staged_files = Git::staged_files(&cli.root)?;
                files.extend(staged_files);
            }
            if changed {
                let changed_files = Git::changed_files(&cli.root)?;
                files.extend(changed_files);
            }
            files.retain(|f| !f.trim().is_empty());
            files.sort();
            files.dedup();

            if files.is_empty() {
                if !json {
                    println!("No files provided or changed to check.");
                }
                return Ok(());
            }

            let mut check = Queries::check_work(
                db.conn(),
                "local_collection",
                &files,
                version.as_deref(),
                env.as_deref(),
            )?;

            // Diff-Aware Risk Suppression: If changes are purely comments or whitespace, suppress to avoid alert fatigue
            if staged || changed {
                for m in &mut check.matches {
                    if !m.acknowledged {
                        let all_trivial = m.matched_paths.iter().all(|path| {
                            Git::file_diff_is_trivial(&cli.root, path, staged).unwrap_or(false)
                        });
                        if all_trivial && !m.matched_paths.is_empty() {
                            m.suppressed = true;
                            m.suppression_reason = Some(
                                "Diff contains only comments or whitespace (suppressed to prevent alert fatigue)".to_string(),
                            );
                        }
                    }
                }
            }

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
                        let status_tag = if m.suppressed {
                            "[SUPPRESSED]"
                        } else {
                            match m.applicability {
                                hyperkb_rs::domain::RiskApplicability::Applies => "[APPLIES]",
                                hyperkb_rs::domain::RiskApplicability::Unknown => "[UNKNOWN]",
                                hyperkb_rs::domain::RiskApplicability::NotApplicable => "[N/A]",
                            }
                        };
                        println!(
                            "  {} {} ({})",
                            status_tag, m.document.title, m.document.path
                        );
                        println!("     Matched: {}", m.matched_paths.join(", "));
                        println!("     Reason:  {}", m.reason);
                        if let Some(ref sup) = m.suppression_reason {
                            println!("     Notice:  {}", sup);
                        } else if let Some(ref ack) = m.acknowledgement {
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
        Some(Commands::InstallHook) => {
            let path = Git::install_pre_commit_hook(&cli.root)?;
            println!("✓ Pre-commit risk interception hook installed successfully at:");
            println!("  {}", path.display());
            println!("\nAll staged commits will now be evaluated against cited open risks.");
        }
        Some(Commands::DraftRisk {
            title,
            rationale,
            owner,
            paths,
            versions,
            environments,
        }) => {
            let draft = RiskWorkflow::draft_risk(
                &cli.root,
                db.conn(),
                "local_collection",
                &title,
                &rationale,
                &owner,
                paths,
                versions,
                environments,
            )?;
            println!("✓ Proposed risk record drafted:");
            println!("  Path:   {}", draft.path);
            println!("  ID:     {}", draft.id);
            println!("  Status: {}", draft.status);
            println!("  Paths:  {}", draft.paths.join(", "));
            println!("\nIndexed and active for pre-edit interception.");
        }
        Some(Commands::Mcp) => {
            let docs_dir = PathBuf::from("../hyperkb/docs");
            if docs_dir.exists() {
                let _ = Scanner::index_directory(db.conn(), &docs_dir, "local_collection");
            } else {
                let _ = Scanner::index_directory(db.conn(), &cli.root, "local_collection");
            }
            McpServer::run_stdio(&cli.root, db.conn(), "local_collection", "default_profile")?;
        }
        Some(Commands::DraftDecision {
            title,
            rationale,
            owner,
            supersedes,
        }) => {
            let actor = Actor::Human { username: owner };
            let draft = DecisionWorkflow::draft_replacement(
                &cli.root,
                db.conn(),
                "local_collection",
                &title,
                &rationale,
                &actor,
                supersedes.as_deref(),
            )?;
            println!("✓ Proposed decision drafted:");
            println!("  Path:   {}", draft.path);
            println!("  ID:     {}", draft.id);
            println!("  Status: {}", draft.status);
            println!("\nAwaiting human review and acceptance before committing.");
        }
        Some(Commands::AcceptDecision {
            path,
            owner,
            supersedes,
        }) => {
            let actor = Actor::Human { username: owner };
            let review = DecisionWorkflow::review_acceptance(
                &cli.root,
                db.conn(),
                &path,
                &actor,
                supersedes.as_deref(),
            )?;
            println!("Accepting decision at {}:", path);
            DecisionWorkflow::accept_decision(&cli.root, db.conn(), "local_collection", &review, Some(&actor))?;
            println!("✓ Decision accepted and indexed as architectural authority.");
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
        Some(Commands::Backup { keep }) => {
            let report = MaintenanceManager::create_backup(
                &cli.root,
                &db,
                "local_collection",
                "default_profile",
                keep,
            )?;
            println!("✓ Backup snapshot created successfully:");
            println!("  Path:    {}", report.path);
            println!("  Kept:    {} snapshot(s)", report.kept);
            println!("  Removed: {} older snapshot(s)", report.removed);
        }
        Some(Commands::ListBackups) => {
            let snapshots = MaintenanceManager::list_backups(&cli.root)?;
            println!("Managed backup snapshots ({} total):", snapshots.len());
            for snap in snapshots {
                println!("  - {}", snap);
            }
        }
        Some(Commands::Compact) => {
            println!("Compacting database and truncating WAL journal...");
            db.compact()?;
            println!("✓ Database compacted successfully.");
        }
        Some(Commands::AcknowledgeRisk {
            risk_id,
            rationale,
            owner,
        }) => {
            let actor = Actor::Human { username: owner };
            let ack = RiskWorkflow::acknowledge_risk(
                &cli.root,
                db.conn(),
                "local_collection",
                &risk_id,
                &actor,
                &rationale,
            )?;
            println!("✓ Risk acknowledged by human principal '{}':", actor.responsible_owner());
            println!("  Path:   {}", ack.path);
            println!("  ID:     {}", ack.id);
            println!("  Status: {}", ack.status);
            println!("\nThis risk is recorded as waived/mitigated and will no longer block check-work or git commits.");
        }
        Some(Commands::Grant { command }) => match command {
            GrantCommands::Issue {
                grantee,
                granted_by,
                actions,
                scope,
                max_diff,
                ttl_hours,
            } => {
                let authorizer = granted_by
                    .or_else(|| std::env::var("USER").ok())
                    .unwrap_or_else(|| "Developer".to_string());

                let allowed_actions: Vec<ActionKind> = actions
                    .split(',')
                    .filter_map(|s| match s.trim().to_ascii_lowercase().as_str() {
                        "accept-decision" | "accept_decision" => Some(ActionKind::AcceptDecision),
                        "acknowledge-risk" | "acknowledge_risk" => Some(ActionKind::AcknowledgeRisk),
                        "propose-decision" | "propose_decision" => Some(ActionKind::ProposeDecision),
                        "auto-repair" | "auto_repair" => Some(ActionKind::AutoRepair),
                        _ => None,
                    })
                    .collect();

                let allowed_scope_patterns: Vec<String> = scope
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                let constraints = GrantConstraints {
                    max_line_diff: Some(max_diff),
                    require_tests_pass: false,
                    allow_supersede: true,
                    expires_at: ttl_hours.map(|h| Utc::now() + chrono::Duration::hours(h as i64)),
                };

                let grant = GrantStore::issue_grant(
                    &cli.root,
                    &grantee,
                    &authorizer,
                    allowed_actions,
                    allowed_scope_patterns,
                    constraints,
                )?;

                println!("✓ AuthorityGrant issued and saved to .hyperkb/grants/{}.json", grant.grant_id);
                println!("  Grant ID:    {}", grant.grant_id);
                println!("  Grantee:     {}", grant.grantee);
                println!("  Granted By:  {} (Responsible Human Principal)", grant.granted_by);
                println!("  Actions:     {:?}", grant.allowed_actions);
                println!("  Scope:       {:?}", grant.allowed_scope_patterns);
                println!("  Max Diff:    {} lines", max_diff);
                if let Some(exp) = grant.constraints.expires_at {
                    println!("  Expires:     {}", exp.to_rfc3339());
                } else {
                    println!("  Expires:     Never");
                }
            }
            GrantCommands::List => {
                let grants = GrantStore::list_grants(&cli.root)?;
                if grants.is_empty() {
                    println!("No authority grants currently registered in .hyperkb/grants.");
                } else {
                    println!("Registered Authority Grants ({}):", grants.len());
                    for g in grants {
                        let expired = g.constraints.expires_at.map_or(false, |exp| Utc::now() > exp);
                        let status_str = if expired { " [EXPIRED]" } else { " [ACTIVE]" };
                        println!(
                            "• {} (Grantee: {}, Principal: {}){}",
                            g.grant_id, g.grantee, g.granted_by, status_str
                        );
                        println!("    Scope:   {:?}", g.allowed_scope_patterns);
                        println!("    Actions: {:?}", g.allowed_actions);
                    }
                }
            }
            GrantCommands::Revoke { grant_id } => {
                let uid = Uuid::parse_str(&grant_id)
                    .map_err(|_| format!("Invalid grant UUID '{}'", grant_id))?;
                if GrantStore::revoke_grant(&cli.root, uid)? {
                    println!("✓ AuthorityGrant '{}' revoked and removed.", uid);
                } else {
                    println!("Grant '{}' not found.", uid);
                }
            }
        },
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
