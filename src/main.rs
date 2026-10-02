use chrono::Utc;
use clap::{Parser, Subcommand};
use hyperkb::core::{
    Archeology, DecisionWorkflow, DirectiveWorkflow, Git, GrantStore, KbLinter,
    MaintenanceManager, RiskWorkflow, Scanner, SessionManager,
};
use hyperkb::domain::{ActionKind, Actor, BrowseOptions, GrantConstraints, RepoManifest};
use hyperkb::storage::{Database, Queries};
use hyperkb::transport::McpServer;
use hyperkb::ui;
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
    /// Get the warm-start context briefing (invariants, risks, churn hotspots, friction warnings)
    Brief {
        /// Optional active authority grant scope pattern
        #[arg(short, long)]
        scope: Option<String>,
    },
    /// View session effectiveness metrics, coding velocity, and review loop oscillations
    Metrics {
        /// Optional specific session ID (defaults to latest session)
        #[arg(short, long)]
        session_id: Option<String>,
        /// Output metrics as JSON
        #[arg(long)]
        json: bool,
    },
    /// Inspect or list agent session records
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },
    /// Manage standing policy directives and invariants
    Directive {
        #[command(subcommand)]
        command: DirectiveCommands,
    },
    /// Manage repository policy taxonomies
    Taxonomy {
        #[command(subcommand)]
        command: TaxonomyCommands,
    },
    /// Audit repository documentation and policy directives for bloat, invalid schemas, and decay
    Audit {
        /// Audit KB documentation (nesting depth, line/word bloat, stale docs, metadata schema)
        #[arg(long)]
        kb: bool,
        /// Audit policy directives (dormant rules, category bloat, staleness)
        #[arg(long)]
        directives: bool,
        /// Output audit results as JSON
        #[arg(long)]
        json: bool,
    },
    /// View or update repository configuration and policy knobs (hyperkb.json)
    Config {
        /// Show current configuration or specific key
        #[arg(short, long)]
        get: Option<String>,
        /// Set a configuration key=value (e.g. -s max_briefing_directives=8, -s theme=nord, -s mouse_enabled=false)
        #[arg(short, long)]
        set: Option<String>,
        /// Output configuration as JSON
        #[arg(long)]
        json: bool,
    },
    /// Print comprehensive HyperKB architecture, methodology, shortcuts, and telemetry documentation
    Docs {
        /// Optional section to display (overview, shortcuts, math, settings, themes, mcp)
        section: Option<String>,
    },
}

#[derive(Subcommand)]
enum DirectiveCommands {
    /// List policy directives
    List {
        /// Optional category filter (architecture, behavior, deployment, security)
        #[arg(short, long)]
        category: Option<String>,
        /// Optional status filter (active, dormant, retired)
        #[arg(short, long)]
        status: Option<String>,
        /// Output directives as JSON
        #[arg(long)]
        json: bool,
    },
    /// Draft a new policy directive
    #[command(alias = "new")]
    Draft {
        /// Directive title (e.g. "Zero Code Comments")
        title: String,
        /// Valid category from repo taxonomy
        #[arg(short, long)]
        category: String,
        /// Human author or accountable engineer
        #[arg(short, long, default_value = "Developer")]
        author: String,
        /// Scoped path patterns (e.g. -s "src/**" or -s "*")
        #[arg(short, long)]
        scope: Vec<String>,
        /// Enforcement mode: check_work, briefing, or manual
        #[arg(short, long, default_value = "check_work")]
        enforcement: String,
        /// ID of an older directive this supersedes
        #[arg(long)]
        supersedes: Option<String>,
        /// Rule description and details
        #[arg(short, long, default_value = "")]
        content: String,
    },
    /// Retire an existing directive
    Retire {
        /// Directive ID (e.g. DIR-...)
        id: String,
    },
    /// Audit directives for bloat (>5 global rules) and stale paths
    Audit {
        /// Output report as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum TaxonomyCommands {
    /// List configured taxonomy categories
    List {
        #[arg(long)]
        json: bool,
    },
    /// Add a new category to the repository taxonomy in hyperkb.json
    Add {
        /// Unique category identifier (slug, e.g. "compliance")
        id: String,
        /// Human-readable display name (e.g. "Compliance & Privacy")
        name: String,
        /// Description of policies that belong to this category
        description: String,
    },
}

#[derive(Subcommand)]
enum SessionCommands {
    /// List recent agent sessions
    List {
        #[arg(short, long, default_value_t = 10)]
        limit: usize,
    },
    /// Show details, events, and scorecard for a specific session
    Show {
        /// Session ID (e.g. sess_01...)
        id: String,
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

            // Diff-Aware Risk Suppression & Comment Hygiene Check
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

                for path in &check.checked_paths {
                    if let Ok(Some(rep)) = Git::check_file_comment_hygiene(&cli.root, path, staged) {
                        if rep.is_excessive {
                            check.hygiene_warnings.push(format!(
                                "Excessive comment density in '{}': {}/{} added lines ({:.1}%) are comments. Keep code self-documenting per Comment Directive.",
                                rep.file_path, rep.comment_lines, rep.added_lines, rep.comment_ratio * 100.0
                            ));
                        }
                    }
                }
            }

            // Lint markdown documentation files for KB anti-bloat & schema compliance
            let docs_dir = cli.root.join(&manifest.docs_root);
            for path in &check.checked_paths {
                if path.ends_with(".md") || path.ends_with(".markdown") {
                    let file_p = cli.root.join(path);
                    if file_p.exists() {
                        if let Ok(issues) = KbLinter::lint_single_file(&file_p, &docs_dir) {
                            for issue in issues {
                                check.hygiene_warnings.push(format!("KB Linter [{}]: {}", path, issue));
                            }
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

                if !check.applicable_directives.is_empty() {
                    println!("📜 Applicable Policy Directives (Rule of 5):");
                    for d in &check.applicable_directives {
                        let scope_tag = if d.is_global() { "global".to_string() } else { d.scope.join(",") };
                        println!("  - [{}] {} ({}) [enforce: {}]", d.category.to_uppercase(), d.title, scope_tag, d.enforcement);
                    }
                    println!();
                }

                if !check.hygiene_warnings.is_empty() {
                    println!("[!] Comment Hygiene Warnings:");
                    for w in &check.hygiene_warnings {
                        println!("  - {}", w);
                    }
                    println!();
                }

                if check.matches.is_empty() {
                    println!("Clean: No cited open risks or path hazards detected.");
                } else {
                    println!("[!] Found {} cited risk(s):", check.matches.len());
                    for m in &check.matches {
                        let status_tag = if m.suppressed {
                            "[SUPPRESSED]"
                        } else {
                            match m.applicability {
                                hyperkb::domain::RiskApplicability::Applies => "[APPLIES]",
                                hyperkb::domain::RiskApplicability::Unknown => "[UNKNOWN]",
                                hyperkb::domain::RiskApplicability::NotApplicable => "[N/A]",
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
        Some(Commands::Brief { scope }) => {
            let briefing = SessionManager::generate_briefing_with_limit(
                db.conn(),
                collection_id,
                scope.as_deref(),
                manifest.settings.max_briefing_directives,
            )
            .map_err(|e| format!("Failed to generate briefing: {}", e))?;
            println!("{}", briefing.formatted_markdown);
        }
        Some(Commands::Metrics { session_id, json }) => {
            let target_id = match session_id {
                Some(id) => id,
                None => {
                    let sessions = Queries::list_sessions(db.conn(), collection_id, 1)?;
                    sessions
                        .into_iter()
                        .next()
                        .map(|s| s.id)
                        .ok_or_else(|| "No agent sessions recorded yet in this repository.".to_string())?
                }
            };
            let scorecard = SessionManager::compute_scorecard(db.conn(), &target_id)
                .map_err(|e| format!("Failed to compute scorecard: {}", e))?;

            if json {
                println!("{}", serde_json::to_string_pretty(&scorecard)?);
            } else {
                println!("=== HyperKB Session Scorecard ===");
                println!("Session ID:              {}", scorecard.session_id);
                println!("Duration:                {}s", scorecard.duration_seconds);
                println!("Tool-to-Edit Ratio:      {:.1} (Healthy: 2.0 - 4.0)", scorecard.tool_to_edit_ratio);
                println!("Coding Effectiveness:    {:.0}%", scorecard.coding_effectiveness * 100.0);
                println!("Knowledge Hit Rate:      {:.0}%", scorecard.knowledge_hit_rate * 100.0);
                println!("Review Loops Detected:   {}", scorecard.review_loops_detected);
                println!("Risks Handled/Prevented: {}", scorecard.risks_handled);
                if !scorecard.friction_hotspots.is_empty() {
                    println!("\nFriction Hotspots (Churn Re-edits):");
                    for path in &scorecard.friction_hotspots {
                        println!("  - {}", path);
                    }
                }
                if !scorecard.zero_hit_queries.is_empty() {
                    println!("\nKnowledge Debt (Zero-Hit Search Queries):");
                    for q in &scorecard.zero_hit_queries {
                        println!("  - \"{}\"", q);
                    }
                }
            }
        }
        Some(Commands::Session { command }) => match command {
            SessionCommands::List { limit } => {
                let sessions = Queries::list_sessions(db.conn(), collection_id, limit)?;
                if sessions.is_empty() {
                    println!("No agent sessions recorded yet in collection '{}'.", collection_id);
                } else {
                    println!("Recent Agent Sessions ({}) for collection '{}':", sessions.len(), collection_id);
                    for s in sessions {
                        println!(
                            "• {} | Agent: {:<12} | Status: {:<9} | Started: {}",
                            s.id, s.agent_id, s.status, s.started_at
                        );
                        println!(
                            "    Tools: {:<3} | Edits: {:<3} | Diff Lines: {:<4} | Loops: {}",
                            s.total_tool_calls, s.total_edits, s.total_diff_lines, s.review_loops
                        );
                    }
                }
            }
            SessionCommands::Show { id } => {
                let session = Queries::get_session(db.conn(), &id)?
                    .ok_or_else(|| format!("Session '{}' not found", id))?;
                let events = Queries::get_session_events(db.conn(), &id)?;
                let scorecard = SessionManager::compute_scorecard(db.conn(), &id)
                    .map_err(|e| format!("Failed to compute scorecard: {}", e))?;

                println!("Session ID:    {}", session.id);
                println!("Agent:         {}", session.agent_id);
                println!("Status:        {}", session.status);
                println!("Started:       {}", session.started_at);
                if let Some(ended) = session.ended_at {
                    println!("Ended:         {}", ended);
                }
                println!("Effectiveness: {:.0}%", scorecard.coding_effectiveness * 100.0);
                println!("Tool Ratio:    {:.1}", scorecard.tool_to_edit_ratio);
                println!("Review Loops:  {}", scorecard.review_loops_detected);
                println!("\nEvent Log ({} events):", events.len());
                for ev in events {
                    println!("  [{}] {:<18} path='{}' query/tool='{}'", ev.timestamp, ev.event_kind, ev.target_path, ev.query_or_tool);
                }
            }
        },
        Some(Commands::Directive { command }) => match command {
            DirectiveCommands::List { category, status, json } => {
                let directives = Queries::list_directives(
                    db.conn(),
                    collection_id,
                    category.as_deref(),
                    status.as_deref(),
                )?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&directives)?);
                } else if directives.is_empty() {
                    println!("No directives found matching the given filters.");
                } else {
                    println!("Policy Directives ({}) in collection '{}':\n", directives.len(), collection_id);
                    for d in directives {
                        let status_tag = match d.status.as_str() {
                            "active" => "[ACTIVE]",
                            "dormant" => "[DORMANT]",
                            "retired" => "[RETIRED]",
                            _ => "[OTHER]",
                        };
                        let scope_str = if d.is_global() { "global".to_string() } else { d.scope.join(", ") };
                        println!("• {} {:<14} [{}] {}", status_tag, d.id, d.category.to_uppercase(), d.title);
                        println!("    Scope: {:<20} | Enforce: {}", scope_str, d.enforcement);
                    }
                }
            }
            DirectiveCommands::Draft {
                title,
                category,
                author,
                scope,
                enforcement,
                supersedes,
                content,
            } => {
                let target_scope = if scope.is_empty() {
                    vec!["*".to_string()]
                } else {
                    scope.clone()
                };
                let dir = DirectiveWorkflow::draft_directive(
                    &cli.root,
                    db.conn(),
                    collection_id,
                    &title,
                    &category,
                    &author,
                    target_scope,
                    &enforcement,
                    supersedes.clone(),
                    &content,
                )?;
                println!("✓ Drafted directive '{}' ({})", dir.title, dir.id);
                println!("  Category:    {}", dir.category);
                println!("  Scope:       {:?}", dir.scope);
                println!("  Enforcement: {}", dir.enforcement);
                println!("  Path:        {}/{}.md", manifest.directives_path, dir.id);
            }
            DirectiveCommands::Retire { id } => {
                let retired = DirectiveWorkflow::retire_directive(&cli.root, db.conn(), &id)?;
                if retired {
                    println!("✓ Directive '{}' is now retired.", id);
                } else {
                    println!("Directive '{}' was not found or already retired.", id);
                }
            }
            DirectiveCommands::Audit { json } => {
                let report = DirectiveWorkflow::audit_directives(&cli.root, db.conn(), collection_id)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    println!("Policy Directives Audit Report for collection '{}':\n", collection_id);
                    println!("  Total Directives:  {}", report.total_directives);
                    println!("  Active Directives: {}", report.active_directives);
                    println!("  Global Rules:      {} (Threshold: ≤ 5)", report.global_count);
                    println!("  Taxonomies Used:   {}", report.taxonomies_used.join(", "));
                    println!();

                    if !report.bloat_warnings.is_empty() {
                        println!("Rule Bloat Warnings:");
                        for w in &report.bloat_warnings {
                            println!("  - {}", w);
                        }
                        println!();
                    }

                    if !report.stale_directives.is_empty() {
                        println!("Stale Scope Warnings:");
                        for w in &report.stale_directives {
                            println!("  - {}", w);
                        }
                        println!();
                    }

                    if report.bloat_warnings.is_empty() && report.stale_directives.is_empty() {
                        println!("Clean: All active directives are scoped, fresh, and within the Rule of 5 threshold.");
                    }
                }
            }
        },
        Some(Commands::Taxonomy { command }) => match command {
            TaxonomyCommands::List { json } => {
                let m = RepoManifest::load_or_default(&cli.root);
                if json {
                    println!("{}", serde_json::to_string_pretty(&m.taxonomy)?);
                } else {
                    println!("Configured Policy Taxonomies ({}):\n", m.taxonomy.categories.len());
                    for cat in &m.taxonomy.categories {
                        println!("• {:<14} - {} ({})", cat.id, cat.label, cat.description);
                    }
                }
            }
            TaxonomyCommands::Add { id, name, description } => {
                let mut m = RepoManifest::load_or_default(&cli.root);
                m.add_category(id.clone(), name, description)
                    .map_err(|e| format!("Failed to add taxonomy: {}", e))?;
                m.save(&cli.root)
                    .map_err(|e| format!("Failed to save hyperkb.json: {}", e))?;
                println!("✓ Added taxonomy category '{}' to hyperkb.json.", id);
            }
        },
        Some(Commands::Audit { kb, directives, json }) => {
            let audit_kb_flag = if !kb && !directives { true } else { kb };
            let audit_directives_flag = if !kb && !directives { true } else { directives };

            let mut kb_report = None;
            let mut dir_report = None;

            if audit_kb_flag {
                let docs_dir = cli.root.join(&manifest.docs_root);
                let report = KbLinter::audit_directory_with_settings(
                    &docs_dir,
                    manifest.settings.audit_max_lines,
                    manifest.settings.audit_max_depth,
                    manifest.settings.stale_days_threshold,
                )?;
                kb_report = Some(report);
            }

            if audit_directives_flag {
                let report = DirectiveWorkflow::audit_directives(&cli.root, db.conn(), collection_id)?;
                dir_report = Some(report);
            }

            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "kb": kb_report,
                        "directives": dir_report,
                    }))?
                );
            } else {
                if let Some(report) = kb_report {
                    println!("=== HyperKB Knowledge Base & Documentation Audit ===");
                    println!("  Scanned Documents: {}", report.total_documents);
                    println!("  Valid Documents:   {}", report.valid_documents);
                    if !report.topics.is_empty() {
                        println!("  Topics / Folders:  {}", report.topics.join(", "));
                    }
                    println!();

                    if !report.bloat_warnings.is_empty() {
                        println!("Document Bloat Warnings (Target: ≤ {} lines / ≤ 2000 words):", manifest.settings.audit_max_lines);
                        for w in &report.bloat_warnings {
                            println!("  - {}", w);
                        }
                        println!();
                    }

                    if !report.depth_warnings.is_empty() {
                        println!("Folder Depth Warnings (Target: ≤ {} levels):", manifest.settings.audit_max_depth);
                        for w in &report.depth_warnings {
                            println!("  - {}", w);
                        }
                        println!();
                    }

                    if !report.schema_errors.is_empty() {
                        println!("Metadata Schema Errors:");
                        for w in &report.schema_errors {
                            println!("  - {}", w);
                        }
                        println!();
                    }

                    if !report.stale_warnings.is_empty() {
                        println!("Stale Proposals (> {} days in proposed status):", manifest.settings.stale_days_threshold);
                        for w in &report.stale_warnings {
                            println!("  - {}", w);
                        }
                        println!();
                    }

                    if report.is_clean() {
                        println!(
                            "Clean: All documents comply with line count (≤ {}), word count (≤ 2000), nesting depth (≤ {}), and frontmatter schema.",
                            manifest.settings.audit_max_lines, manifest.settings.audit_max_depth
                        );
                        println!();
                    }
                }

                if let Some(report) = dir_report {
                    println!("=== HyperKB Policy Directives Audit ===");
                    println!("  Total Directives:  {}", report.total_directives);
                    println!("  Active Directives: {}", report.active_directives);
                    println!("  Global Rules:      {} (Configured Threshold: ≤ {})", report.global_count, manifest.settings.max_briefing_directives);
                    println!("  Taxonomies Used:   {}", report.taxonomies_used.join(", "));
                    println!();

                    if !report.bloat_warnings.is_empty() {
                        println!("Rule Bloat Warnings:");
                        for w in &report.bloat_warnings {
                            println!("  - {}", w);
                        }
                        println!();
                    }

                    if !report.stale_directives.is_empty() {
                        println!("Stale Scope Warnings:");
                        for w in &report.stale_directives {
                            println!("  - {}", w);
                        }
                        println!();
                    }

                    if report.bloat_warnings.is_empty() && report.stale_directives.is_empty() {
                        println!("Clean: All active directives are scoped, fresh, and within the Rule of {} threshold.", manifest.settings.max_briefing_directives);
                    }
                }
            }
        }
        Some(Commands::Config { get, set, json }) => {
            let mut manifest = RepoManifest::load_or_default(&cli.root);
            if let Some(set_str) = set {
                if let Some((key, val)) = set_str.split_once('=') {
                    let key = key.trim();
                    let val = val.trim();
                    match key {
                        "max_briefing_directives" | "directives_ceiling" => {
                            let n: usize = val.parse().map_err(|_| format!("Invalid integer value '{}'", val))?;
                            manifest.settings.max_briefing_directives = n.clamp(1, 20);
                            println!("✓ Updated max_briefing_directives to {}", manifest.settings.max_briefing_directives);
                        }
                        "stale_days_threshold" | "stale_days" => {
                            let n: i64 = val.parse().map_err(|_| format!("Invalid integer value '{}'", val))?;
                            manifest.settings.stale_days_threshold = n.clamp(7, 365);
                            println!("✓ Updated stale_days_threshold to {} days", manifest.settings.stale_days_threshold);
                        }
                        "audit_max_lines" | "max_lines" => {
                            let n: usize = val.parse().map_err(|_| format!("Invalid integer value '{}'", val))?;
                            manifest.settings.audit_max_lines = n.clamp(50, 1000);
                            println!("✓ Updated audit_max_lines to {} lines", manifest.settings.audit_max_lines);
                        }
                        "audit_max_depth" | "max_depth" => {
                            let n: usize = val.parse().map_err(|_| format!("Invalid integer value '{}'", val))?;
                            manifest.settings.audit_max_depth = n.clamp(1, 8);
                            println!("✓ Updated audit_max_depth to {} levels", manifest.settings.audit_max_depth);
                        }
                        "theme" => {
                            let normalized = val.to_lowercase();
                            if !["cyberpunk", "modern", "nord", "tokyonight", "light"].contains(&normalized.as_str()) {
                                return Err(format!("Invalid theme '{}'. Available themes: cyberpunk, modern, nord, tokyonight, light", val).into());
                            }
                            manifest.settings.theme = normalized;
                            println!("✓ Updated theme to '{}'", manifest.settings.theme);
                        }
                        "mouse_enabled" | "mouse" => {
                            let b: bool = match val.to_lowercase().as_str() {
                                "true" | "1" | "yes" | "on" => true,
                                "false" | "0" | "no" | "off" => false,
                                _ => return Err(format!("Invalid boolean '{}'. Use true/false or on/off", val).into()),
                            };
                            manifest.settings.mouse_enabled = b;
                            println!("✓ Updated mouse_enabled to {}", manifest.settings.mouse_enabled);
                        }
                        _ => {
                            return Err(format!("Unknown configuration key '{}'. Available keys:\n  - max_briefing_directives\n  - stale_days_threshold\n  - audit_max_lines\n  - audit_max_depth\n  - theme\n  - mouse_enabled", key).into());
                        }
                    }
                    manifest.save(&cli.root)?;
                } else {
                    return Err("Expected format: key=value (e.g. -s max_briefing_directives=8)".to_string().into());
                }
            } else if json {
                println!("{}", serde_json::to_string_pretty(&manifest.settings)?);
            } else if let Some(key) = get {
                match key.as_str() {
                    "max_briefing_directives" => println!("{}", manifest.settings.max_briefing_directives),
                    "stale_days_threshold" => println!("{}", manifest.settings.stale_days_threshold),
                    "audit_max_lines" => println!("{}", manifest.settings.audit_max_lines),
                    "audit_max_depth" => println!("{}", manifest.settings.audit_max_depth),
                    "theme" => println!("{}", manifest.settings.theme),
                    "mouse_enabled" => println!("{}", manifest.settings.mouse_enabled),
                    _ => return Err(format!("Unknown configuration key '{}'", key).into()),
                }
            } else {
                println!("=== HyperKB Policy Configuration (hyperkb.json) ===");
                println!("  max_briefing_directives : {} (Rule of N directive ceiling)", manifest.settings.max_briefing_directives);
                println!("  stale_days_threshold    : {} days (KB freshness horizon)", manifest.settings.stale_days_threshold);
                println!("  audit_max_lines         : {} lines (Doc length limit)", manifest.settings.audit_max_lines);
                println!("  audit_max_depth         : {} levels (Folder nesting depth)", manifest.settings.audit_max_depth);
                println!("  theme                   : {} (TUI color theme)", manifest.settings.theme);
                println!("  mouse_enabled           : {} (Terminal mouse capture)", manifest.settings.mouse_enabled);
                println!();
                println!("To modify a setting: hyperkb config -s key=value");
            }
        }
        Some(Commands::Docs { section }) => {
            print_docs(section.as_deref());
        }
        None => {
            // Index existing docs in workspace so the TUI opens with real knowledge ready to browse
            let docs_dir = cli.root.join(&manifest.docs_root);
            if docs_dir.exists() {
                let _ = Scanner::index_directory(db.conn(), &docs_dir, collection_id);
            } else {
                let _ = Scanner::index_directory(db.conn(), &cli.root, collection_id);
            }

            // Launch the full-screen Ratatui TUI
            ui::run(&cli.root, &db, collection_id, profile_id)?;
        }
    }

    Ok(())
}

fn print_docs(section: Option<&str>) {
    let s = section.unwrap_or("all").to_lowercase();

    if s == "all" || s == "overview" {
        println!("================================================================================");
        println!("  HyperKB: Deterministic Knowledge Architecture for Autonomous AI Coding Agents ");
        println!("================================================================================");
        println!();
        println!("HyperKB eliminates context bloat, hallucinated conventions, and PR rejection loops");
        println!("by establishing an active, local knowledge graph backed by SQLite and Ratatui.");
        println!();
        println!("CORE PHILOSOPHY & PILLARS:");
        println!("  1. Zero-Ceremony Context Injection");
        println!("     Instead of querying bloated multi-thousand line prompt rules, AI agents call");
        println!("     'get_session_briefing' to receive active architectural invariants, high-priority");
        println!("     risks, recent churn hotspots, and friction warnings in 1 deterministic roundtrip.");
        println!();
        println!("  2. Pre-Commit Enforcement Gate");
        println!("     'hyperkb check' runs during git pre-commit to evaluate modified files against");
        println!("     active architectural invariants and unacknowledged risks. Violations block git commit.");
        println!();
        println!("  3. Session Quality Telemetry");
        println!("     Every agent coding session records tool calls, edits, diff volume, and review oscillations.");
        println!("     Run quality is tracked using an honest 0–100 heuristic scoring review loops, tool friction,");
        println!("     and directive compliance without hype or buzzwords.");
        println!();
    }

    if s == "all" || s == "shortcuts" {
        println!("================================================================================");
        println!("  HyperKB TUI Navigation & Shortcuts Reference                                 ");
        println!("================================================================================");
        println!("GLOBAL SHORTCUTS:");
        println!("  [1] - [5]         Switch between tabs (Work, Knowledge, Directives, Agents, Settings)");
        println!("  [/] / [Space]     Open anchored Universal Command Dock (slash commands & AI prompts)");
        println!("  [Tab]             Toggle focus between Left (List/Tree) and Right (Detail/Scorecard) pane");
        println!("  [↑] / [↓] / [j/k] Navigate through items, trees, or settings");
        println!("  [PgDn] / [PgUp]   Page through documents or long lists (Space also advances Reader)");
        println!("  [Ctrl+d/Ctrl+u]   Half-page scroll down / up");
        println!("  [m]               Toggle Mouse Mode: ON (TUI clicks) vs OFF (Terminal text selection)");
        println!("  [T]               Cycle Color Theme (Cyberpunk, Modern, Nord, TokyoNight, Light)");
        println!("  [?] / [F1]        Open interactive in-app quick reference modal (Esc / q to close)");
        println!("  [q] / [Ctrl+c]    Quit HyperKB cleanly, restoring terminal state");
        println!();
        println!("TERMINAL COPY / PASTE SELECTION:");
        println!("  • Mouse Mode ON:  Left-click navigates tabs, categories, tree nodes, and setting knobs.");
        println!("  • Mouse Mode OFF: Terminal mouse capture is released. Drag with your mouse to natively");
        println!("                    highlight and copy text (Cmd+C / Ctrl+Shift+C) directly to your OS clipboard.");
        println!("  • macOS Shortcut: Hold Option (⌥) while dragging to select text EVEN WHEN mouse mode is ON!");
        println!("  • Linux / iTerm:  Hold Shift while dragging to bypass terminal mouse capture.");
        println!();
        println!("VIEW-SPECIFIC CONTROLS:");
        println!("  [Knowledge]       [t] Toggle Tree / List view | [c] Cycle category filter | [Enter] Open reader");
        println!("  [Directives]      [c] Cycle policy taxonomy   | [r] Retire selected directive | [Enter] Read rule");
        println!("  [Agents]          [h] Cycle harness filter    | [e] Toggle Scorecard breakdown | [g] Toggle Grants");
        println!("  [Settings]        [←] / [→] / [h/l] / [-/+] Adjust knob value | [Enter] Save to hyperkb.json");
        println!("  [Reader]          [v] Toggle Raw Markdown / Formatted view | [Esc] Return to Knowledge list");
        println!();
    }

    if s == "all" || s == "math" || s == "validation" {
        println!("================================================================================");
        println!("  Session Quality Score: Heuristic Specification & Breakdown                    ");
        println!("================================================================================");
        println!("HyperKB does not rely on opaque LLM ratings or subjective scores. Every session");
        println!("quality rating is derived from a transparent, pragmatic heuristic:");
        println!();
        println!("FORMULA:");
        println!("  S = clamp(100% - P_loops - P_friction - P_thrash + B_hazard, 5%, 100%)");
        println!();
        println!("PARAMETER DEFINITIONS & PENALTY MATRICES:");
        println!("  • P_loops    = min(review_loops × 15%, 45%)");
        println!("                 Cyclic review oscillations where an agent repeatedly edits and fails review.");
        println!("  • P_friction = 15% if first_pass_clean == false");
        println!("                 Deducted when initial generated code requires test/compile fix iterations.");
        println!("  • P_thrash   = 20% if (tool_calls / edits) > 10.0");
        println!("                 10% if (tool_calls / edits) > 6.0");
        println!("                 Penalizes aimless file reading and exploratory thrashing before making edits.");
        println!("  • B_hazard   = +10% if risks_prevented > 0");
        println!("                 Rewarded when the agent proactively honors active architectural invariants.");
        println!();
        println!("ADVANCED EFFICIENCY TELEMETRY:");
        println!("  • Hazard Interception Rate: (risks_prevented / total_risks_cited) × 100%");
        println!("  • Code Churn Density:       total_diff_lines / total_edits (measures surgical precision)");
        println!("  • Action Velocity:          (total_edits / total_tool_calls) × 100% (measures directness)");
        println!("  • Throughput Velocity:      (total_diff_lines / duration_seconds) × 60 lines/min");
        println!();
    }

    if s == "all" || s == "settings" || s == "config" {
        println!("================================================================================");
        println!("  Configurable Policy Knobs (hyperkb.json)                                      ");
        println!("================================================================================");
        println!("All policy settings are persisted in repository root 'hyperkb.json' and configurable");
        println!("via the TUI [5] Settings tab or the CLI ('hyperkb config -s key=value'):");
        println!();
        println!("  1. max_briefing_directives (Default: 5 | Range: 1 - 20)");
        println!("     Controls the maximum number of active directives injected into AI session briefings.");
        println!("     Prevents LLM prompt context degradation (the 'Rule of 5').");
        println!();
        println!("  2. stale_days_threshold (Default: 90 days | Range: 7 - 365 days)");
        println!("     Threshold before unverified proposals/ADRs trigger stale warnings during 'hyperkb audit'.");
        println!();
        println!("  3. audit_max_lines (Default: 250 lines | Range: 50 - 1000 lines)");
        println!("     Hard length ceiling for knowledge base markdown files to prevent bloated documentation.");
        println!();
        println!("  4. audit_max_depth (Default: 3 levels | Range: 1 - 8 levels)");
        println!("     Maximum directory nesting depth allowed for knowledge documents.");
        println!();
        println!("  5. theme (Default: cyberpunk | Options: cyberpunk, modern, nord, tokyonight, light)");
        println!("     Sets the active visual color scheme for the Ratatui TUI.");
        println!();
        println!("  6. mouse_enabled (Default: true | Options: true, false)");
        println!("     Enables terminal mouse capture on launch. Set to false for native terminal text copying.");
        println!();
    }

    if s == "all" || s == "mcp" {
        println!("================================================================================");
        println!("  Model Context Protocol (MCP) Integration                                      ");
        println!("================================================================================");
        println!("HyperKB provides a full-featured native MCP stdio server for Claude, Gemini, and Cursor:");
        println!();
        println!("CONFIGURATION (claude_desktop_config.json):");
        println!("  {{");
        println!("    \"mcpServers\": {{");
        println!("      \"hyperkb\": {{");
        println!("        \"command\": \"/path/to/hyperkb\",");
        println!("        \"args\": [\"mcp\"],");
        println!("        \"cwd\": \"/path/to/your/project\"");
        println!("      }}");
        println!("    }}");
        println!("  }}");
        println!();
        println!("EXPOSED MCP TOOLS:");
        println!("  • get_session_briefing  : Zero-ceremony warm-start context briefing.");
        println!("  • check_work            : Validate modified files against invariants and risks.");
        println!("  • draft_directive       : Create new scoped policy directive.");
        println!("  • retire_directive      : Retire an obsolete policy directive.");
        println!("  • audit_directives      : Check directives for bloat and stale file patterns.");
        println!("  • record_session_event  : Log tool execution, file edits, and review loops.");
        println!("  • get_scorecard         : Retrieve session effectiveness and telemetry scorecard.");
        println!("================================================================================");
    }
}
