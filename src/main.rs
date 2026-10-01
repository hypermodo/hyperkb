use chrono::Utc;
use clap::{Parser, Subcommand};
use hyperkb_rs::core::{
    Archeology, DecisionWorkflow, Git, GrantStore, MaintenanceManager, RiskWorkflow, Scanner,
};
use hyperkb_rs::domain::{ActionKind, Actor, BrowseOptions, GrantConstraints, RepoManifest};
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
    /// Initialize a new HyperKB repository manifest (hyperkb.json) and standard directories
    Init {
        /// Project name (defaults to current folder name)
        #[arg(short, long)]
        name: Option<String>,
        /// Primary collection ID
        #[arg(short, long)]
        collection: Option<String>,
    },
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
    /// Discover historical incident hotspots and draft proactive risk cards from Git history
    Bootstrap {
        /// Maximum number of candidate risk cards to draft
        #[arg(short, long, default_value_t = 5)]
        limit: usize,
        /// Maximum commits in Git history to analyze
        #[arg(long, default_value_t = 500)]
        max_commits: usize,
        /// Show proposed risk candidates without writing them to disk
        #[arg(long)]
        dry_run: bool,
        /// Responsible human owner for the drafted risks (defaults to git user or "Developer")
        #[arg(short, long)]
        owner: Option<String>,
    },
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

    let manifest = RepoManifest::load_or_default(&cli.root);
    let collection_id = &manifest.collection_id;
    let profile_id = &manifest.default_profile;

    let db_path = cli.root.join(".hyperkb").join("hyperkb.db");
    let db = Database::open_or_create(&db_path, collection_id, profile_id)?;

    match cli.command {
        Some(Commands::Init { name, collection }) => {
            let (init_manifest, path) =
                RepoManifest::init(&cli.root, name.as_deref(), collection.as_deref())?;
            println!("✓ Initialized HyperKB repository manifest:");
            println!("  Path:          {}", path.display());
            println!("  Collection ID: {}", init_manifest.collection_id);
            println!("  Project Name:  {}", init_manifest.name);
            println!("  Decisions:     {}", init_manifest.decisions_path);
            println!("  Risks:         {}", init_manifest.risks_path);

            match Git::install_pre_commit_hook(&cli.root) {
                Ok(hook_path) => {
                    println!("✓ Pre-commit risk interception hook installed at {}", hook_path.display())
                }
                Err(e) => println!("Note: Git pre-commit hook skipped ({})", e),
            }
        }
        Some(Commands::Index { dir }) => {
            println!("Indexing Markdown documents in: {}", dir.display());
            let report = Scanner::index_directory(db.conn(), &dir, collection_id)?;
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
                &[collection_id.clone()],
                profile_id,
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
            let (docs, total) = Queries::browse(db.conn(), &[collection_id.clone()], &opts)?;
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
                collection_id,
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
        Some(Commands::Bootstrap {
            limit,
            max_commits,
            dry_run,
            owner,
        }) => {
            println!("Performing Git history archeology across up to {} commits...", max_commits);
            let report = Archeology::bootstrap(
                &cli.root,
                db.conn(),
                collection_id,
                limit,
                max_commits,
                dry_run,
                owner.as_deref(),
            )?;

            println!(
                "Analyzed {} commits; found {} historical incident commits.",
                report.analyzed_commits, report.incident_commits
            );

            if report.candidates.is_empty() {
                println!("No recurring incident hotspots detected in Git history matching incident criteria.");
            } else {
                let mode_str = if dry_run { "Candidate Risk Hotspots (DRY RUN):" } else { "Created Risk Cards:" };
                println!("\n{}", mode_str);
                for (i, c) in report.candidates.iter().enumerate() {
                    println!("\n[{}] {}", i + 1, c.title);
                    println!("    Affected Paths: {:?}", c.affected_paths);
                    println!("    Incident Count: {} commits", c.incident_count);
                    let cited = c.cited_commits.iter().map(|s| &s[..s.len().min(8)]).collect::<Vec<_>>().join(", ");
                    println!("    Cited Commits:  {}", cited);
                    if let Some(ref path) = c.created_path {
                        println!("    Created File:   {}", path);
                    }
                }
                if dry_run {
                    println!("\nRun without --dry-run to generate and index these risk records into docs/risks/.");
                } else {
                    println!("\n✓ {} risk cards drafted and indexed into docs/risks/.", report.candidates.len());
                }
            }
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
                collection_id,
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
            let docs_dir = cli.root.join(&manifest.docs_root);
            if docs_dir.exists() {
                let _ = Scanner::index_directory(db.conn(), &docs_dir, collection_id);
            } else {
                let _ = Scanner::index_directory(db.conn(), &cli.root, collection_id);
            }
            McpServer::run_stdio(&cli.root, db.conn(), collection_id, profile_id)?;
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
                collection_id,
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
            DecisionWorkflow::accept_decision(&cli.root, db.conn(), collection_id, &review, Some(&actor))?;
            println!("✓ Decision accepted and indexed as architectural authority.");
        }
        Some(Commands::Remember { title, content }) => {
            let id = uuid::Uuid::now_v7().to_string();
            Queries::remember(
                db.conn(),
                &id,
                profile_id,
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
                collection_id,
                profile_id,
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
                collection_id,
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
            let docs_dir = cli.root.join(&manifest.docs_root);
            if docs_dir.exists() {
                let _ = Scanner::index_directory(db.conn(), &docs_dir, collection_id);
            } else {
                let _ = Scanner::index_directory(db.conn(), &cli.root, collection_id);
            }

            // Launch the full-screen Ratatui TUI
            ui::run(&db)?;
        }
    }

    Ok(())
}
