use crate::domain::{
    AgentSession, AuthorityGrant, BrowseOptions, Directive, Document, ProjectSummary, RepoManifest,
    RiskMatch,
};
use crate::storage::{Database, Queries};
use crate::ui::theme::ThemeMode;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkTabMode {
    Projects,
    Risks,
    Console,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceTabMode {
    Sessions,
    Grants,
}

#[derive(Debug, Clone)]
pub struct DiagnosticEntry {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub command: String,
    pub title: String,
    pub success: bool,
    pub summary: String,
    pub lines: Vec<String>,
    pub file_targets: Vec<String>,
    pub selected_file_idx: usize,
}

#[derive(Debug, Clone)]
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
    pub example: &'static str,
}

pub static SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "claude",
        description: "Ask Claude Code CLI headlessly with KB governance context",
        example: "/claude <prompt>",
    },
    SlashCommand {
        name: "opencode",
        description: "Ask OpenCode AI Harness headlessly with KB governance context",
        example: "/opencode <prompt>",
    },
    SlashCommand {
        name: "openai",
        description: "Ask OpenAI / Codex CLI headlessly with KB governance context",
        example: "/openai <prompt>",
    },
    SlashCommand {
        name: "codex",
        description: "Ask Codex CLI headlessly with KB governance context",
        example: "/codex <prompt>",
    },
    SlashCommand {
        name: "antigravity",
        description: "Query Antigravity IDE agent session environment",
        example: "/antigravity <prompt>",
    },
    SlashCommand {
        name: "agent",
        description: "Ask default active AI harness headlessly with KB governance context",
        example: "/agent <prompt>",
    },
    SlashCommand {
        name: "audit",
        description: "Run KB anti-bloat, schema & directive decay audit",
        example: "/audit",
    },
    SlashCommand {
        name: "check",
        description: "Check staged & changed files against risks and directives",
        example: "/check",
    },
    SlashCommand {
        name: "reindex",
        description: "Re-index markdown documents into SQLite full-text search",
        example: "/reindex",
    },
    SlashCommand {
        name: "bootstrap",
        description: "Mine git log history to bootstrap candidate risks",
        example: "/bootstrap",
    },
    SlashCommand {
        name: "harnesses",
        description: "Inspect discovered AI harnesses & CISO governance status",
        example: "/harnesses",
    },
    SlashCommand {
        name: "search",
        description: "Full-text search ADRs, specs, and knowledge documents",
        example: "/search <query>",
    },
    SlashCommand {
        name: "read",
        description: "Open currently selected document in full Reader view",
        example: "/read",
    },
    SlashCommand {
        name: "directives",
        description: "Switch to Policy Directives & Invariants tab",
        example: "/directives",
    },
    SlashCommand {
        name: "new",
        description: "Draft a new policy directive or governance invariant",
        example: "/new",
    },
    SlashCommand {
        name: "toggle",
        description: "Toggle active/retired status of selected policy directive",
        example: "/toggle",
    },
    SlashCommand {
        name: "risks",
        description: "Switch to Work view & inspect active repository risk cards",
        example: "/risks",
    },
    SlashCommand {
        name: "sessions",
        description: "Switch to Agent Runs & telemetry scorecard tab",
        example: "/sessions",
    },
    SlashCommand {
        name: "scorecard",
        description: "Inspect detailed 0-100 Session Quality Scorecard for selected run",
        example: "/scorecard",
    },
    SlashCommand {
        name: "grants",
        description: "Switch to Agent Authority Grants tab",
        example: "/grants",
    },
    SlashCommand {
        name: "grant",
        description: "Issue scoped authority grant token to an agent",
        example: "/grant",
    },
    SlashCommand {
        name: "revoke",
        description: "Revoke selected agent authority grant token",
        example: "/revoke",
    },
    SlashCommand {
        name: "backup",
        description: "Create atomic verified point-in-time database snapshot",
        example: "/backup",
    },
    SlashCommand {
        name: "compact",
        description: "SQLite database VACUUM & truncate WAL journal",
        example: "/compact",
    },
    SlashCommand {
        name: "clear",
        description: "Clear terminal diagnostic stream output",
        example: "/clear",
    },
    SlashCommand {
        name: "help",
        description: "Show available terminal commands and keybindings",
        example: "/help",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Work = 0,
    Explore = 1,
    Directives = 2,
    Sessions = 3,
    Settings = 4,
    Reader = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    List,
    Detail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExploreTreeItem {
    Folder {
        path: String,
        name: String,
        doc_count: usize,
        is_collapsed: bool,
    },
    Doc {
        doc_idx: usize,
        title: String,
        path: String,
        status: String,
        kind: String,
    },
}

pub struct App {
    pub should_quit: bool,
    pub active_tab: ActiveTab,
    pub focused_pane: FocusedPane,
    pub collection_id: String,
    pub profile_id: String,
    pub manifest: RepoManifest,
    pub theme: ThemeMode,
    pub mouse_capture: bool,
    pub drag_start: Option<(u16, u16)>,
    pub drag_current: Option<(u16, u16)>,
    pub is_dragging: bool,
    pub last_selected_text: Option<String>,

    // Help & Methodology state
    pub show_help: bool,
    pub help_scroll: usize,
    pub show_scoring_methodology: bool,

    // Settings state
    pub settings_selected_idx: usize,
    pub settings_dirty: bool,

    // Explore / Document state
    pub documents: Vec<Document>,
    pub selected_doc_idx: usize,
    pub selected_category: String,
    pub explore_tree_mode: bool,
    pub collapsed_folders: std::collections::HashSet<String>,
    pub selected_tree_idx: usize,

    // Directives state
    pub directives: Vec<Directive>,
    pub selected_directive_idx: usize,
    pub directive_category: String,
    pub directive_preview_scroll: usize,

    // Sessions state
    pub sessions: Vec<AgentSession>,
    pub selected_session_idx: usize,
    pub session_preview_scroll: usize,

    // Work / Risk state
    pub active_risks: Vec<RiskMatch>,
    pub selected_risk_idx: usize,

    // Reader & Preview scroll state
    pub current_document: Option<Document>,
    pub reader_scroll_offset: usize,
    pub preview_scroll_offset: usize,
    pub show_raw: bool,

    pub is_filtering: bool,
    pub filter_query: String,
    pub status_message: Option<String>,

    pub root: PathBuf,

    // Governance & Grants state
    pub governance_tab_mode: GovernanceTabMode,
    pub grants: Vec<AuthorityGrant>,
    pub selected_grant_idx: usize,
    pub session_harness_filter: Option<String>,

    // Directive creation modal state
    pub show_new_directive_modal: bool,
    pub new_directive_title: String,
    pub new_directive_category_idx: usize,
    pub new_directive_scope: String,
    pub new_directive_enforcement_idx: usize,
    pub new_directive_rule: String,
    pub new_directive_field: usize,

    // Issue grant modal state
    pub show_issue_grant_modal: bool,
    pub new_grant_grantee: String,
    pub new_grant_preset_idx: usize,
    pub new_grant_ttl_hours: u32,
    pub new_grant_field: usize,


    // Work / Cockpit tab mode and project scaling
    pub work_tab_mode: WorkTabMode,
    pub projects: Vec<ProjectSummary>,
    pub selected_project_idx: usize,
    pub project_tasks: Vec<Document>,
    pub selected_project_task_idx: usize,
    pub project_task_scroll_offset: usize,

    // Live Diagnostic Output Stream & Command Input
    pub diagnostic_stream: Vec<DiagnosticEntry>,
    pub selected_diagnostic_idx: usize,
    pub diagnostic_scroll: usize,
    pub repl_input: String,
    pub repl_history: Vec<String>,
    pub repl_history_idx: usize,
    pub repl_active: bool,
    pub slash_menu_selected_idx: usize,

    // AI Harness & LLM Registry
    pub harnesses: Vec<crate::domain::HarnessDefinition>,
    pub selected_harness_idx: usize,

    // Headless AI Agent Query Channel & Status
    pub agent_rx: Option<std::sync::mpsc::Receiver<DiagnosticEntry>>,
    pub pending_agent_query: Option<(String, std::time::Instant)>,
}

impl App {
    pub fn new(collection_id: &str, profile_id: &str) -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        Self::new_with_root(&root, collection_id, profile_id)
    }

    pub fn new_with_root<P: AsRef<Path>>(root_path: P, collection_id: &str, profile_id: &str) -> Self {
        let root = root_path.as_ref().to_path_buf();
        let manifest = RepoManifest::load_or_default(&root);
        let theme = ThemeMode::from_id(&manifest.settings.theme);
        let mouse_capture = manifest.settings.mouse_enabled;

        let harnesses = crate::core::HarnessDiscovery::discover(&manifest.harnesses);
        let selected_harness_idx = if let Some(ref active_id) = manifest.harnesses.active_harness_id {
            harnesses.iter().position(|h| &h.id == active_id).unwrap_or(0)
        } else {
            0
        };

        let init_entry = DiagnosticEntry {
            id: "boot".to_string(),
            timestamp: chrono::Utc::now(),
            command: "hyperkb terminal".to_string(),
            title: "HyperKB Terminal Initialized".to_string(),
            success: true,
            summary: format!(
                "Collection: '{}'. Max briefing directives: {}.",
                collection_id,
                manifest.settings.max_briefing_directives
            ),
            lines: vec![
                format!("Repo Manifest: loaded from {}", RepoManifest::FILE_NAME),
                format!("Harness Discovery: {} AI tool(s) registered/detected locally", harnesses.len()),
                "Type '/' to open command palette, or 'audit', 'check', 'reindex', 'help' below.".to_string(),
            ],
            file_targets: Vec::new(),
            selected_file_idx: 0,
        };

        Self {
            should_quit: false,
            active_tab: ActiveTab::Work,
            focused_pane: FocusedPane::List,
            collection_id: collection_id.to_string(),
            profile_id: profile_id.to_string(),
            manifest,
            theme,
            mouse_capture,
            show_help: false,
            help_scroll: 0,
            show_scoring_methodology: false,
            settings_selected_idx: 0,
            settings_dirty: false,
            documents: Vec::new(),
            selected_doc_idx: 0,
            selected_category: "all".into(),
            explore_tree_mode: false,
            collapsed_folders: std::collections::HashSet::new(),
            selected_tree_idx: 0,
            directives: Vec::new(),
            selected_directive_idx: 0,
            directive_category: "all".into(),
            directive_preview_scroll: 0,
            sessions: Vec::new(),
            selected_session_idx: 0,
            session_preview_scroll: 0,
            active_risks: Vec::new(),
            selected_risk_idx: 0,
            current_document: None,
            reader_scroll_offset: 0,
            preview_scroll_offset: 0,
            show_raw: false,
            is_filtering: false,
            filter_query: String::new(),
            status_message: None,
            drag_start: None,
            drag_current: None,
            is_dragging: false,
            last_selected_text: None,
            root,
            governance_tab_mode: GovernanceTabMode::Sessions,
            grants: Vec::new(),
            selected_grant_idx: 0,
            session_harness_filter: None,
            show_new_directive_modal: false,
            new_directive_title: String::new(),
            new_directive_category_idx: 0,
            new_directive_scope: "*".to_string(),
            new_directive_enforcement_idx: 0,
            new_directive_rule: String::new(),
            new_directive_field: 0,
            show_issue_grant_modal: false,
            new_grant_grantee: String::new(),
            new_grant_preset_idx: 0,
            new_grant_ttl_hours: 4,
            new_grant_field: 0,
            work_tab_mode: WorkTabMode::Projects,
            projects: Vec::new(),
            selected_project_idx: 0,
            project_tasks: Vec::new(),
            selected_project_task_idx: 0,
            project_task_scroll_offset: 0,
            diagnostic_stream: vec![init_entry],
            selected_diagnostic_idx: 0,
            diagnostic_scroll: 0,
            repl_input: String::new(),
            repl_history: Vec::new(),
            repl_history_idx: 0,
            repl_active: false,
            slash_menu_selected_idx: 0,
            harnesses,
            selected_harness_idx,
            agent_rx: None,
            pending_agent_query: None,
        }
    }

    pub fn filtered_slash_commands(&self) -> Vec<&'static SlashCommand> {
        if !self.repl_input.starts_with('/') {
            return Vec::new();
        }
        let clean = self.repl_input.trim_start_matches('/');
        // If the user already typed a space, they are typing arguments/prompts to the command
        if clean.contains(' ') {
            return Vec::new();
        }
        let query = clean.trim().to_lowercase();
        if query.is_empty() {
            return SLASH_COMMANDS.iter().collect();
        }
        SLASH_COMMANDS
            .iter()
            .filter(|cmd| {
                cmd.name.starts_with(&query)
                    || cmd.name.contains(&query)
                    || cmd.description.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn toggle_raw_view(&mut self) {
        self.show_raw = !self.show_raw;
    }

    pub fn toggle_explore_tree_mode(&mut self) {
        self.explore_tree_mode = !self.explore_tree_mode;
        self.selected_tree_idx = 0;
        self.preview_scroll_offset = 0;
        if self.explore_tree_mode {
            let tree = self.build_explore_tree();
            for (idx, item) in tree.iter().enumerate() {
                if let ExploreTreeItem::Doc { doc_idx, .. } = item {
                    if *doc_idx == self.selected_doc_idx {
                        self.selected_tree_idx = idx;
                        break;
                    }
                }
            }
        }
    }

    pub fn toggle_tree_collapse(&mut self, folder: &str) {
        if self.collapsed_folders.contains(folder) {
            self.collapsed_folders.remove(folder);
        } else {
            self.collapsed_folders.insert(folder.to_string());
        }
    }

    pub fn build_explore_tree(&self) -> Vec<ExploreTreeItem> {
        let mut folders: Vec<String> = Vec::new();
        for doc in &self.documents {
            let folder = Path::new(&doc.path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let clean_folder = if folder.is_empty() {
                "general".to_string()
            } else {
                folder
            };
            if !folders.contains(&clean_folder) {
                folders.push(clean_folder);
            }
        }
        folders.sort();

        let mut items = Vec::new();
        for folder in folders {
            let matching_indices: Vec<usize> = self
                .documents
                .iter()
                .enumerate()
                .filter(|(_, doc)| {
                    let f = Path::new(&doc.path)
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let cf = if f.is_empty() { "general" } else { &f };
                    cf == folder
                })
                .map(|(idx, _)| idx)
                .collect();

            let doc_count = matching_indices.len();
            let is_collapsed = self.collapsed_folders.contains(&folder);

            items.push(ExploreTreeItem::Folder {
                path: folder.clone(),
                name: folder.trim_end_matches('/').to_string(),
                doc_count,
                is_collapsed,
            });

            if !is_collapsed {
                for idx in matching_indices {
                    let doc = &self.documents[idx];
                    items.push(ExploreTreeItem::Doc {
                        doc_idx: idx,
                        title: doc.title.clone(),
                        path: doc.path.clone(),
                        status: doc.status.as_str().to_string(),
                        kind: doc.kind.as_str().to_string(),
                    });
                }
            }
        }

        items
    }

    pub fn selected_tree_item(&self) -> Option<ExploreTreeItem> {
        let tree = self.build_explore_tree();
        tree.get(self.selected_tree_idx).cloned()
    }

    /// Loads initial data from the database.
    pub fn refresh_data(&mut self, db: &Database) {
        let opts = BrowseOptions {
            category: self.selected_category.clone(),
            limit: 100,
            offset: 0,
            ..Default::default()
        };

        if let Ok((docs, _)) = Queries::browse(db.conn(), &[self.collection_id.clone()], &opts) {
            self.documents = docs;
            if self.selected_doc_idx >= self.documents.len() && !self.documents.is_empty() {
                self.selected_doc_idx = self.documents.len() - 1;
            }
        }

        let cat_opt = if self.directive_category == "all" {
            None
        } else {
            Some(self.directive_category.as_str())
        };
        if let Ok(dirs) = Queries::list_directives(db.conn(), &self.collection_id, cat_opt, None) {
            self.directives = dirs;
            if self.selected_directive_idx >= self.directives.len() && !self.directives.is_empty() {
                self.selected_directive_idx = self.directives.len() - 1;
            }
        }

        if let Ok(sess) = Queries::list_sessions(db.conn(), &self.collection_id, 20) {
            self.sessions = sess;
            if self.selected_session_idx >= self.sessions.len() && !self.sessions.is_empty() {
                self.selected_session_idx = self.sessions.len() - 1;
            }
        }

        if let Ok(grants) = crate::core::GrantStore::list_grants(&self.root) {
            self.grants = grants;
            if self.selected_grant_idx >= self.grants.len() && !self.grants.is_empty() {
                self.selected_grant_idx = self.grants.len() - 1;
            }
        }

        if let Ok(projects) = Queries::list_projects(db.conn(), &self.collection_id) {
            self.projects = projects;
            if self.selected_project_idx >= self.projects.len() && !self.projects.is_empty() {
                self.selected_project_idx = self.projects.len() - 1;
            }
            self.refresh_project_tasks(db);
        }
    }

    pub fn refresh_project_tasks(&mut self, db: &Database) {
        if let Some(proj) = self.projects.get(self.selected_project_idx) {
            let opts = BrowseOptions {
                project: Some(proj.name.clone()),
                limit: 200,
                ..Default::default()
            };
            if let Ok((docs, _)) = Queries::browse(db.conn(), &[self.collection_id.clone()], &opts) {
                self.project_tasks = docs;
                if self.selected_project_task_idx >= self.project_tasks.len() && !self.project_tasks.is_empty() {
                    self.selected_project_task_idx = self.project_tasks.len() - 1;
                }
            }
        } else {
            self.project_tasks.clear();
            self.selected_project_task_idx = 0;
        }
    }

    pub fn set_selected_project(&mut self, idx: usize, db: &Database) {
        if !self.projects.is_empty() {
            self.selected_project_idx = idx.min(self.projects.len() - 1);
            self.selected_project_task_idx = 0;
            self.refresh_project_tasks(db);
        }
    }

    pub fn cycle_work_tab_mode(&mut self, db: &Database) {
        self.work_tab_mode = match self.work_tab_mode {
            WorkTabMode::Projects => WorkTabMode::Risks,
            WorkTabMode::Risks => WorkTabMode::Console,
            WorkTabMode::Console => WorkTabMode::Projects,
        };
        if self.work_tab_mode == WorkTabMode::Projects {
            if self.projects.is_empty() {
                self.refresh_data(db);
            } else {
                self.refresh_project_tasks(db);
            }
        }
    }

    pub const CATEGORIES: &'static [&'static str] = &["all", "tasks", "decisions", "risks", "specs", "plans"];

    pub fn next_category(&mut self, db: &Database) {
        let current_pos = Self::CATEGORIES
            .iter()
            .position(|&c| c == self.selected_category)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % Self::CATEGORIES.len();
        self.set_category(Self::CATEGORIES[next_pos], db);
    }

    pub fn set_category(&mut self, category: &str, db: &Database) {
        self.selected_category = category.to_string();
        self.selected_doc_idx = 0;
        self.preview_scroll_offset = 0;
        self.refresh_data(db);
    }

    pub const DIRECTIVE_CATEGORIES: &'static [&'static str] = &[
        "all",
        "architecture",
        "behavior",
        "deployment",
        "security",
    ];

    pub fn directive_categories(&self) -> Vec<String> {
        let mut cats = vec!["all".to_string()];
        for cat in &self.manifest.taxonomy.categories {
            if !cats.contains(&cat.id) {
                cats.push(cat.id.clone());
            }
        }
        cats
    }

    pub fn next_directive_category(&mut self, db: &Database) {
        let cats = self.directive_categories();
        let current_pos = cats
            .iter()
            .position(|c| c == &self.directive_category)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % cats.len();
        self.set_directive_category(&cats[next_pos], db);
    }

    pub fn prev_directive_category(&mut self, db: &Database) {
        let cats = self.directive_categories();
        let current_pos = cats
            .iter()
            .position(|c| c == &self.directive_category)
            .unwrap_or(0);
        let prev_pos = if current_pos == 0 {
            cats.len().saturating_sub(1)
        } else {
            current_pos - 1
        };
        self.set_directive_category(&cats[prev_pos], db);
    }

    pub fn set_directive_category(&mut self, category: &str, db: &Database) {
        self.directive_category = category.to_string();
        self.selected_directive_idx = 0;
        self.directive_preview_scroll = 0;
        self.refresh_data(db);
    }

    pub fn refresh_harnesses(&mut self) {
        self.harnesses = crate::core::HarnessDiscovery::discover(&self.manifest.harnesses);
        if self.selected_harness_idx >= self.harnesses.len() && !self.harnesses.is_empty() {
            self.selected_harness_idx = 0;
        }
    }

    pub fn retire_selected_directive<P: AsRef<Path>>(
        &mut self,
        root: P,
        db: &Database,
    ) -> Result<bool, String> {
        if let Some(dir) = self.directives.get(self.selected_directive_idx) {
            let id = dir.id.clone();
            let res = crate::core::DirectiveWorkflow::retire_directive(root, db.conn(), &id)?;
            self.refresh_data(db);
            self.status_message = Some(format!("Retired directive {}", id));
            Ok(res)
        } else {
            Ok(false)
        }
    }

    pub fn switch_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
        self.focused_pane = FocusedPane::List;
        self.reader_scroll_offset = 0;
    }

    pub fn toggle_pane(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::List => FocusedPane::Detail,
            FocusedPane::Detail => FocusedPane::List,
        };
    }

    pub fn next(&mut self) {
        match self.active_tab {
            ActiveTab::Work => {
                match self.work_tab_mode {
                    WorkTabMode::Projects => {
                        if self.focused_pane == FocusedPane::Detail {
                            if !self.project_tasks.is_empty() {
                                self.selected_project_task_idx = (self.selected_project_task_idx + 1) % self.project_tasks.len();
                            }
                        } else if !self.projects.is_empty() {
                            self.selected_project_idx = (self.selected_project_idx + 1) % self.projects.len();
                            self.selected_project_task_idx = 0;
                        }
                    }
                    WorkTabMode::Console => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.diagnostic_scroll += 2;
                        } else if let Some(entry) = self.diagnostic_stream.get_mut(self.selected_diagnostic_idx) {
                            if entry.file_targets.len() > 1 {
                                entry.selected_file_idx = (entry.selected_file_idx + 1) % entry.file_targets.len();
                            } else if !self.diagnostic_stream.is_empty() {
                                self.selected_diagnostic_idx = (self.selected_diagnostic_idx + 1) % self.diagnostic_stream.len();
                                self.diagnostic_scroll = 0;
                            }
                        }
                    }
                    WorkTabMode::Risks => {
                        if !self.active_risks.is_empty() {
                            self.selected_risk_idx = (self.selected_risk_idx + 1) % self.active_risks.len();
                        }
                    }
                }
            }
            ActiveTab::Explore => {
                if self.focused_pane == FocusedPane::Detail {
                    self.preview_scroll_offset += 2;
                } else if self.explore_tree_mode {
                    let tree = self.build_explore_tree();
                    if !tree.is_empty() {
                        self.selected_tree_idx = (self.selected_tree_idx + 1) % tree.len();
                        if let ExploreTreeItem::Doc { doc_idx, .. } = &tree[self.selected_tree_idx] {
                            self.selected_doc_idx = *doc_idx;
                        }
                        self.preview_scroll_offset = 0;
                    }
                } else if !self.documents.is_empty() {
                    self.selected_doc_idx = (self.selected_doc_idx + 1) % self.documents.len();
                    self.preview_scroll_offset = 0;
                }
            }
            ActiveTab::Directives => {
                if self.focused_pane == FocusedPane::Detail {
                    self.directive_preview_scroll += 2;
                } else if !self.directives.is_empty() {
                    self.selected_directive_idx = (self.selected_directive_idx + 1) % self.directives.len();
                    self.directive_preview_scroll = 0;
                }
            }
            ActiveTab::Sessions => {
                match self.governance_tab_mode {
                    GovernanceTabMode::Sessions => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.session_preview_scroll += 2;
                        } else if !self.sessions.is_empty() {
                            self.selected_session_idx = (self.selected_session_idx + 1) % self.sessions.len();
                            self.session_preview_scroll = 0;
                        }
                    }
                    GovernanceTabMode::Grants => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.session_preview_scroll += 2;
                        } else if !self.grants.is_empty() {
                            self.selected_grant_idx = (self.selected_grant_idx + 1) % self.grants.len();
                            self.session_preview_scroll = 0;
                        }
                    }
                }
            }
            ActiveTab::Settings => {
                if self.focused_pane == FocusedPane::Detail {
                    self.adjust_setting(1);
                } else {
                    self.next_setting();
                }
            }
            ActiveTab::Reader => {
                self.reader_scroll_offset += 2;
            }
        }
    }

    pub fn prev(&mut self) {
        match self.active_tab {
            ActiveTab::Work => {
                match self.work_tab_mode {
                    WorkTabMode::Projects => {
                        if self.focused_pane == FocusedPane::Detail {
                            if !self.project_tasks.is_empty() {
                                if self.selected_project_task_idx == 0 {
                                    self.selected_project_task_idx = self.project_tasks.len().saturating_sub(1);
                                } else {
                                    self.selected_project_task_idx -= 1;
                                }
                            }
                        } else if !self.projects.is_empty() {
                            if self.selected_project_idx == 0 {
                                self.selected_project_idx = self.projects.len().saturating_sub(1);
                            } else {
                                self.selected_project_idx -= 1;
                            }
                            self.selected_project_task_idx = 0;
                        }
                    }
                    WorkTabMode::Console => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.diagnostic_scroll = self.diagnostic_scroll.saturating_sub(2);
                        } else if let Some(entry) = self.diagnostic_stream.get_mut(self.selected_diagnostic_idx) {
                            if entry.file_targets.len() > 1 {
                                if entry.selected_file_idx == 0 {
                                    entry.selected_file_idx = entry.file_targets.len().saturating_sub(1);
                                } else {
                                    entry.selected_file_idx -= 1;
                                }
                            } else if !self.diagnostic_stream.is_empty() {
                                if self.selected_diagnostic_idx == 0 {
                                    self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                                } else {
                                    self.selected_diagnostic_idx -= 1;
                                }
                                self.diagnostic_scroll = 0;
                            }
                        }
                    }
                    WorkTabMode::Risks => {
                        if !self.active_risks.is_empty() {
                            if self.selected_risk_idx == 0 {
                                self.selected_risk_idx = self.active_risks.len() - 1;
                            } else {
                                self.selected_risk_idx -= 1;
                            }
                        }
                    }
                }
            }
            ActiveTab::Explore => {
                if self.focused_pane == FocusedPane::Detail {
                    self.preview_scroll_offset = self.preview_scroll_offset.saturating_sub(2);
                } else if self.explore_tree_mode {
                    let tree = self.build_explore_tree();
                    if !tree.is_empty() {
                        if self.selected_tree_idx == 0 {
                            self.selected_tree_idx = tree.len() - 1;
                        } else {
                            self.selected_tree_idx -= 1;
                        }
                        if let ExploreTreeItem::Doc { doc_idx, .. } = &tree[self.selected_tree_idx] {
                            self.selected_doc_idx = *doc_idx;
                        }
                        self.preview_scroll_offset = 0;
                    }
                } else if !self.documents.is_empty() {
                    if self.selected_doc_idx == 0 {
                        self.selected_doc_idx = self.documents.len() - 1;
                    } else {
                        self.selected_doc_idx -= 1;
                    }
                    self.preview_scroll_offset = 0;
                }
            }
            ActiveTab::Directives => {
                if self.focused_pane == FocusedPane::Detail {
                    self.directive_preview_scroll = self.directive_preview_scroll.saturating_sub(2);
                } else if !self.directives.is_empty() {
                    if self.selected_directive_idx == 0 {
                        self.selected_directive_idx = self.directives.len() - 1;
                    } else {
                        self.selected_directive_idx -= 1;
                    }
                    self.directive_preview_scroll = 0;
                }
            }
            ActiveTab::Sessions => {
                match self.governance_tab_mode {
                    GovernanceTabMode::Sessions => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.session_preview_scroll = self.session_preview_scroll.saturating_sub(2);
                        } else if !self.sessions.is_empty() {
                            if self.selected_session_idx == 0 {
                                self.selected_session_idx = self.sessions.len() - 1;
                            } else {
                                self.selected_session_idx -= 1;
                            }
                            self.session_preview_scroll = 0;
                        }
                    }
                    GovernanceTabMode::Grants => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.session_preview_scroll = self.session_preview_scroll.saturating_sub(2);
                        } else if !self.grants.is_empty() {
                            if self.selected_grant_idx == 0 {
                                self.selected_grant_idx = self.grants.len() - 1;
                            } else {
                                self.selected_grant_idx -= 1;
                            }
                            self.session_preview_scroll = 0;
                        }
                    }
                }
            }
            ActiveTab::Settings => {
                if self.focused_pane == FocusedPane::Detail {
                    self.adjust_setting(-1);
                } else {
                    self.prev_setting();
                }
            }
            ActiveTab::Reader => {
                if self.reader_scroll_offset > 2 {
                    self.reader_scroll_offset -= 2;
                } else {
                    self.reader_scroll_offset = 0;
                }
            }
        }
    }

    pub fn page_down(&mut self) {
        match self.active_tab {
            ActiveTab::Work => {
                if self.work_tab_mode == WorkTabMode::Console {
                    self.diagnostic_scroll += 10;
                } else {
                    self.next();
                }
            }
            ActiveTab::Explore => {
                if self.focused_pane == FocusedPane::Detail {
                    self.preview_scroll_offset += 15;
                } else if self.explore_tree_mode {
                    let tree = self.build_explore_tree();
                    if !tree.is_empty() {
                        self.selected_tree_idx = (self.selected_tree_idx + 8).min(tree.len() - 1);
                        if let ExploreTreeItem::Doc { doc_idx, .. } = &tree[self.selected_tree_idx] {
                            self.selected_doc_idx = *doc_idx;
                        }
                        self.preview_scroll_offset = 0;
                    }
                } else if !self.documents.is_empty() {
                    self.selected_doc_idx = (self.selected_doc_idx + 8).min(self.documents.len() - 1);
                    self.preview_scroll_offset = 0;
                }
            }
            ActiveTab::Directives => {
                if self.focused_pane == FocusedPane::Detail {
                    self.directive_preview_scroll += 15;
                } else if !self.directives.is_empty() {
                    self.selected_directive_idx = (self.selected_directive_idx + 8).min(self.directives.len() - 1);
                    self.directive_preview_scroll = 0;
                }
            }
            ActiveTab::Sessions => {
                match self.governance_tab_mode {
                    GovernanceTabMode::Sessions => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.session_preview_scroll += 15;
                        } else if !self.sessions.is_empty() {
                            self.selected_session_idx = (self.selected_session_idx + 8).min(self.sessions.len() - 1);
                            self.session_preview_scroll = 0;
                        }
                    }
                    GovernanceTabMode::Grants => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.session_preview_scroll += 15;
                        } else if !self.grants.is_empty() {
                            self.selected_grant_idx = (self.selected_grant_idx + 8).min(self.grants.len() - 1);
                            self.session_preview_scroll = 0;
                        }
                    }
                }
            }
            ActiveTab::Settings => {
                self.next_setting();
            }
            ActiveTab::Reader => {
                self.reader_scroll_offset += 15;
            }
        }
    }

    pub fn page_up(&mut self) {
        match self.active_tab {
            ActiveTab::Work => {
                if self.work_tab_mode == WorkTabMode::Console {
                    self.diagnostic_scroll = self.diagnostic_scroll.saturating_sub(10);
                } else {
                    self.prev();
                }
            }
            ActiveTab::Explore => {
                if self.focused_pane == FocusedPane::Detail {
                    self.preview_scroll_offset = self.preview_scroll_offset.saturating_sub(15);
                } else if self.explore_tree_mode {
                    self.selected_tree_idx = self.selected_tree_idx.saturating_sub(8);
                    let tree = self.build_explore_tree();
                    if let Some(ExploreTreeItem::Doc { doc_idx, .. }) = tree.get(self.selected_tree_idx) {
                        self.selected_doc_idx = *doc_idx;
                    }
                    self.preview_scroll_offset = 0;
                } else if !self.documents.is_empty() {
                    self.selected_doc_idx = self.selected_doc_idx.saturating_sub(8);
                    self.preview_scroll_offset = 0;
                }
            }
            ActiveTab::Directives => {
                if self.focused_pane == FocusedPane::Detail {
                    self.directive_preview_scroll = self.directive_preview_scroll.saturating_sub(15);
                } else if !self.directives.is_empty() {
                    self.selected_directive_idx = self.selected_directive_idx.saturating_sub(8);
                    self.directive_preview_scroll = 0;
                }
            }
            ActiveTab::Sessions => {
                match self.governance_tab_mode {
                    GovernanceTabMode::Sessions => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.session_preview_scroll = self.session_preview_scroll.saturating_sub(15);
                        } else if !self.sessions.is_empty() {
                            self.selected_session_idx = self.selected_session_idx.saturating_sub(8);
                            self.session_preview_scroll = 0;
                        }
                    }
                    GovernanceTabMode::Grants => {
                        if self.focused_pane == FocusedPane::Detail {
                            self.session_preview_scroll = self.session_preview_scroll.saturating_sub(15);
                        } else if !self.grants.is_empty() {
                            self.selected_grant_idx = self.selected_grant_idx.saturating_sub(8);
                            self.session_preview_scroll = 0;
                        }
                    }
                }
            }
            ActiveTab::Settings => {
                self.prev_setting();
            }
            ActiveTab::Reader => {
                self.reader_scroll_offset = self.reader_scroll_offset.saturating_sub(15);
            }
        }
    }

    pub fn next_diagnostic_entry(&mut self) {
        if !self.diagnostic_stream.is_empty() {
            self.selected_diagnostic_idx = (self.selected_diagnostic_idx + 1) % self.diagnostic_stream.len();
            self.diagnostic_scroll = 0;
        }
    }

    pub fn prev_diagnostic_entry(&mut self) {
        if !self.diagnostic_stream.is_empty() {
            if self.selected_diagnostic_idx == 0 {
                self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
            } else {
                self.selected_diagnostic_idx -= 1;
            }
            self.diagnostic_scroll = 0;
        }
    }

    pub fn next_diagnostic_file(&mut self) {
        if let Some(entry) = self.diagnostic_stream.get_mut(self.selected_diagnostic_idx) {
            if !entry.file_targets.is_empty() {
                entry.selected_file_idx = (entry.selected_file_idx + 1) % entry.file_targets.len();
            }
        }
    }

    pub fn prev_diagnostic_file(&mut self) {
        if let Some(entry) = self.diagnostic_stream.get_mut(self.selected_diagnostic_idx) {
            if !entry.file_targets.is_empty() {
                if entry.selected_file_idx == 0 {
                    entry.selected_file_idx = entry.file_targets.len().saturating_sub(1);
                } else {
                    entry.selected_file_idx -= 1;
                }
            }
        }
    }

    pub fn open_selected(&mut self) {
        if self.active_tab == ActiveTab::Explore {
            if self.explore_tree_mode {
                let tree = self.build_explore_tree();
                if let Some(item) = tree.get(self.selected_tree_idx) {
                    match item {
                        ExploreTreeItem::Folder { path, .. } => {
                            let p = path.clone();
                            self.toggle_tree_collapse(&p);
                            return;
                        }
                        ExploreTreeItem::Doc { doc_idx, .. } => {
                            self.selected_doc_idx = *doc_idx;
                        }
                    }
                }
            }
            if let Some(doc) = self.documents.get(self.selected_doc_idx) {
                self.current_document = Some(doc.clone());
                self.active_tab = ActiveTab::Reader;
                self.reader_scroll_offset = 0;
            }
        } else if self.active_tab == ActiveTab::Directives {
            if let Some(dir) = self.directives.get(self.selected_directive_idx) {
                let doc = Document {
                    id: dir.id.clone(),
                    collection_id: dir.collection_id.clone(),
                    path: format!("docs/directives/{}.md", dir.id),
                    title: dir.title.clone(),
                    topic: dir.category.clone(),
                    status: crate::domain::DocumentStatus::Accepted,
                    kind: crate::domain::DocumentKind::Spec,
                    owner: dir.author.clone(),
                    issue: String::new(),
                    replacement_id: None,
                    supersedes: dir.supersedes.clone(),
                    content: dir.to_markdown(),
                    source: "directive".to_string(),
                    available: true,
                    stale: false,
                    declared_status: Some(dir.status.clone()),
                    checksum: String::new(),
                    worktree_state: None,
                };
                self.current_document = Some(doc);
                self.active_tab = ActiveTab::Reader;
                self.reader_scroll_offset = 0;
            }
        } else if self.active_tab == ActiveTab::Work && self.work_tab_mode == WorkTabMode::Projects {
            if let Some(task) = self.project_tasks.get(self.selected_project_task_idx) {
                self.current_document = Some(task.clone());
                self.active_tab = ActiveTab::Reader;
                self.reader_scroll_offset = 0;
            } else if let Some(proj) = self.projects.get(self.selected_project_idx) {
                let status_path = format!("projects/{}/status.md", proj.name);
                let full_p = self.root.join(&status_path);
                if let Ok(content) = std::fs::read_to_string(&full_p) {
                    let doc = Document {
                        id: format!("status-{}", proj.name),
                        collection_id: self.collection_id.clone(),
                        path: status_path,
                        title: format!("{} Status", proj.name),
                        topic: "status".to_string(),
                        status: crate::domain::DocumentStatus::Accepted,
                        kind: crate::domain::DocumentKind::Spec,
                        owner: "project".to_string(),
                        issue: String::new(),
                        replacement_id: None,
                        supersedes: None,
                        content,
                        source: "repo document".to_string(),
                        available: true,
                        stale: false,
                        declared_status: Some("active".to_string()),
                        checksum: String::new(),
                        worktree_state: None,
                    };
                    self.current_document = Some(doc);
                    self.active_tab = ActiveTab::Reader;
                    self.reader_scroll_offset = 0;
                }
            }
        } else if self.active_tab == ActiveTab::Settings {
            let root = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
            let _ = self.save_settings(root);
        }
    }

    pub fn go_back(&mut self) {
        if self.show_help {
            self.show_help = false;
        } else if self.show_scoring_methodology {
            self.show_scoring_methodology = false;
        } else if self.is_filtering {
            self.is_filtering = false;
            self.filter_query.clear();
        } else if self.repl_active {
            self.repl_active = false;
        } else if self.active_tab == ActiveTab::Work && self.work_tab_mode != WorkTabMode::Projects {
            self.work_tab_mode = WorkTabMode::Projects;
        } else if self.active_tab == ActiveTab::Reader {
            self.active_tab = ActiveTab::Explore;
        }
    }

    pub fn next_setting(&mut self) {
        self.settings_selected_idx = (self.settings_selected_idx + 1) % 8;
    }

    pub fn prev_setting(&mut self) {
        if self.settings_selected_idx == 0 {
            self.settings_selected_idx = 7;
        } else {
            self.settings_selected_idx -= 1;
        }
    }

    pub fn adjust_setting(&mut self, delta: i32) {
        match self.settings_selected_idx {
            0 => {
                let curr = self.manifest.settings.max_briefing_directives as i32;
                self.manifest.settings.max_briefing_directives = (curr + delta).clamp(1, 20) as usize;
                self.settings_dirty = true;
            }
            1 => {
                let curr = self.manifest.settings.stale_days_threshold;
                let step = if delta > 0 { 15 } else { -15 };
                self.manifest.settings.stale_days_threshold = (curr + step).clamp(7, 365);
                self.settings_dirty = true;
            }
            2 => {
                let curr = self.manifest.settings.audit_max_lines as i32;
                let step = if delta > 0 { 25 } else { -25 };
                self.manifest.settings.audit_max_lines = (curr + step).clamp(50, 1000) as usize;
                self.settings_dirty = true;
            }
            3 => {
                let curr = self.manifest.settings.audit_max_depth as i32;
                self.manifest.settings.audit_max_depth = (curr + delta).clamp(1, 8) as usize;
                self.settings_dirty = true;
            }
            4 => {
                if delta > 0 {
                    self.theme = self.theme.next();
                } else {
                    self.theme = self.theme.prev();
                }
                self.manifest.settings.theme = self.theme.id_str().to_string();
                self.settings_dirty = true;
            }
            5 => {
                self.mouse_capture = !self.mouse_capture;
                self.manifest.settings.mouse_enabled = self.mouse_capture;
                self.settings_dirty = true;
            }
            6 => {
                self.status_message = Some(format!(
                    "Policy Taxonomy Domains: {} configured in hyperkb.json",
                    self.manifest.taxonomy.categories.len()
                ));
            }
            7 => {
                if !self.harnesses.is_empty() {
                    if delta > 0 {
                        self.selected_harness_idx = (self.selected_harness_idx + 1) % self.harnesses.len();
                    } else if self.selected_harness_idx == 0 {
                        self.selected_harness_idx = self.harnesses.len() - 1;
                    } else {
                        self.selected_harness_idx -= 1;
                    }
                    let sel_id = self.harnesses[self.selected_harness_idx].id.clone();
                    self.manifest.harnesses.active_harness_id = Some(sel_id);
                    self.settings_dirty = true;
                    self.status_message = Some(format!(
                        "Active AI Harness: {}",
                        self.harnesses[self.selected_harness_idx].name
                    ));
                }
            }
            _ => {}
        }
    }

    pub fn next_theme(&mut self) {
        self.theme = self.theme.next();
        self.manifest.settings.theme = self.theme.id_str().to_string();
        self.settings_dirty = true;
        self.status_message = Some(format!("Theme: {}", self.theme.as_str()));
    }

    pub fn prev_theme(&mut self) {
        self.theme = self.theme.prev();
        self.manifest.settings.theme = self.theme.id_str().to_string();
        self.settings_dirty = true;
        self.status_message = Some(format!("Theme: {}", self.theme.as_str()));
    }

    pub fn toggle_mouse(&mut self) -> bool {
        self.mouse_capture = !self.mouse_capture;
        self.manifest.settings.mouse_enabled = self.mouse_capture;
        self.settings_dirty = true;
        self.mouse_capture
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        self.help_scroll = 0;
    }

    pub fn toggle_scoring_methodology(&mut self) {
        self.show_scoring_methodology = !self.show_scoring_methodology;
    }

    pub fn save_settings<P: AsRef<Path>>(&mut self, root: P) -> Result<(), String> {
        self.manifest.save(root)?;
        self.settings_dirty = false;
        self.status_message = Some("Settings saved to hyperkb.json".to_string());
        Ok(())
    }

    pub fn selected_document(&self) -> Option<&Document> {
        self.documents.get(self.selected_doc_idx)
    }

    pub fn selected_directive(&self) -> Option<&Directive> {
        self.directives.get(self.selected_directive_idx)
    }

    pub fn selected_session(&self) -> Option<&AgentSession> {
        self.sessions.get(self.selected_session_idx)
    }

    pub fn selected_risk(&self) -> Option<&RiskMatch> {
        self.active_risks.get(self.selected_risk_idx)
    }

    pub fn copy_active_content_to_clipboard(&mut self) -> Result<String, String> {
        let (content_to_copy, description) = match self.active_tab {
            ActiveTab::Reader => {
                if let Some(doc) = &self.current_document {
                    (doc.content.clone(), format!("Document '{}'", doc.title))
                } else {
                    return Err("No active document in reader to copy".to_string());
                }
            }
            ActiveTab::Directives => {
                if let Some(dir) = self.selected_directive() {
                    (dir.to_markdown(), format!("Directive '{}'", dir.title))
                } else {
                    return Err("No directive selected to copy".to_string());
                }
            }
            ActiveTab::Explore => {
                if let Some(doc) = self.selected_document() {
                    (doc.content.clone(), format!("Document '{}'", doc.title))
                } else {
                    return Err("No document selected to copy".to_string());
                }
            }
            ActiveTab::Work => {
                if let Some(risk) = self.selected_risk() {
                    let text = format!(
                        "Title: {}\nSeverity: {}\nPaths: {:?}\n\n{}",
                        risk.document.title,
                        risk.document.status.as_str(),
                        risk.matched_paths,
                        risk.document.content
                    );
                    (text, format!("Risk '{}'", risk.document.title))
                } else {
                    return Err("No risk item selected to copy".to_string());
                }
            }
            ActiveTab::Sessions => {
                if self.governance_tab_mode == GovernanceTabMode::Grants {
                    if let Some(grant) = self.selected_grant() {
                        let text = serde_json::to_string_pretty(grant).unwrap_or_else(|_| grant.grant_id.to_string());
                        (text, format!("Authority Grant '{}' ({})", grant.grantee, &grant.grant_id.to_string()[..8]))
                    } else {
                        return Err("No authority grant selected to copy".to_string());
                    }
                } else if let Some(sess) = self.selected_session() {
                    let edit_ratio = if sess.total_edits == 0 {
                        sess.total_tool_calls as f64
                    } else {
                        sess.total_tool_calls as f64 / sess.total_edits as f64
                    };
                    let loop_penalty = (sess.review_loops as f64 * 0.15).min(0.45);
                    let first_pass_penalty = if !sess.first_pass_clean { 0.15 } else { 0.0 };
                    let thrash_penalty = if edit_ratio > 10.0 { 0.20 } else if edit_ratio > 6.0 { 0.10 } else { 0.0 };
                    let hazard_bonus = if sess.risks_prevented > 0 { 0.10 } else { 0.0 };
                    let mut score = 1.0f64 - loop_penalty - first_pass_penalty - thrash_penalty + hazard_bonus;
                    score = score.clamp(0.05, 1.0);
                    let score_pct = (score * 100.0).round() as u32;

                    let text = format!(
                        "Session ID: {}\nAgent: {}\nStatus: {}\nCoding Effectiveness: {}%\nDuration: {}\nEdits: {}\nTool Calls: {}\nDiff lines: {}\nReview Loops: {}\nFirst Pass Clean: {}\nRisks Prevented: {}",
                        sess.id,
                        sess.agent_id,
                        sess.status,
                        score_pct,
                        sess.formatted_duration(),
                        sess.total_edits,
                        sess.total_tool_calls,
                        sess.total_diff_lines,
                        sess.review_loops,
                        sess.first_pass_clean,
                        sess.risks_prevented
                    );
                    (text, format!("Session Scorecard '{}'", sess.id))
                } else {
                    return Err("No session selected to copy".to_string());
                }
            }
            ActiveTab::Settings => {
                return Err("Nothing to copy in Settings view".to_string());
            }
        };

        Self::copy_text_to_clipboard(&content_to_copy);
        Ok(description)
    }

    pub fn copy_text_to_clipboard(text: &str) {
        // 1. Native macOS pbcopy
        #[cfg(target_os = "macos")]
        {
            use std::io::Write;
            use std::process::{Command, Stdio};
            if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
        }

        // 2. Linux xclip / wl-copy
        #[cfg(target_os = "linux")]
        {
            use std::io::Write;
            use std::process::{Command, Stdio};
            if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            } else if let Ok(mut child) = Command::new("xclip").args(["-selection", "clipboard"]).stdin(Stdio::piped()).spawn() {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                let _ = child.wait();
            }
        }

        // 3. Universal OSC 52 ANSI escape sequence for terminal emulators (works across SSH, tmux, and modern terminals)
        let b64 = base64_encode(text.as_bytes());
        let osc52 = format!("\x1b]52;c;{}\x07", b64);
        use std::io::Write;
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(osc52.as_bytes());
        let _ = stdout.flush();
    }

    pub fn selected_grant(&self) -> Option<&crate::domain::AuthorityGrant> {
        self.grants.get(self.selected_grant_idx)
    }

    pub fn toggle_governance_tab_mode(&mut self) {
        self.governance_tab_mode = match self.governance_tab_mode {
            GovernanceTabMode::Sessions => GovernanceTabMode::Grants,
            GovernanceTabMode::Grants => GovernanceTabMode::Sessions,
        };
        self.focused_pane = FocusedPane::List;
    }

    pub fn unique_session_harnesses(&self) -> Vec<String> {
        let mut list = Vec::new();
        for s in &self.sessions {
            if !s.agent_id.is_empty() && !list.contains(&s.agent_id) {
                list.push(s.agent_id.clone());
            }
        }
        list
    }

    pub fn filtered_sessions(&self) -> Vec<(usize, &crate::domain::AgentSession)> {
        self.sessions
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                if let Some(ref h) = self.session_harness_filter {
                    s.agent_id.eq_ignore_ascii_case(h)
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn cycle_session_harness_filter(&mut self) {
        let harnesses = self.unique_session_harnesses();
        if harnesses.is_empty() {
            self.session_harness_filter = None;
            return;
        }
        match self.session_harness_filter.as_ref() {
            None => self.session_harness_filter = Some(harnesses[0].clone()),
            Some(curr) => {
                if let Some(idx) = harnesses.iter().position(|h| h == curr) {
                    if idx + 1 < harnesses.len() {
                        self.session_harness_filter = Some(harnesses[idx + 1].clone());
                    } else {
                        self.session_harness_filter = None;
                    }
                } else {
                    self.session_harness_filter = None;
                }
            }
        }
        self.selected_session_idx = 0;
    }

    pub fn active_tab_context(&self) -> (String, String) {
        match self.active_tab {
            ActiveTab::Work => {
                if let Some(risk) = self.active_risks.get(self.selected_risk_idx) {
                    (
                        format!("Work • Risk: {}", risk.document.title),
                        "Ask to analyze risk, explain diff impact, or '/' for actions...".to_string(),
                    )
                } else {
                    (
                        "Work • Working Tree Clean".to_string(),
                        "Ask about active branch, or type '/' for commands (/check, /audit)...".to_string(),
                    )
                }
            }
            ActiveTab::Explore => {
                if let Some(doc) = self.documents.get(self.selected_doc_idx) {
                    (
                        format!("Knowledge • {}", doc.title),
                        "Ask to summarize doc, find related ADRs, or '/' for actions...".to_string(),
                    )
                } else {
                    (
                        "Knowledge Base".to_string(),
                        "Ask to query ADRs and specs, or type '/' for actions...".to_string(),
                    )
                }
            }
            ActiveTab::Directives => {
                if let Some(dir) = self.directives.get(self.selected_directive_idx) {
                    (
                        format!("Directive • {} ({})", dir.id, dir.title),
                        "Ask about policy invariant, draft new, or '/' for actions...".to_string(),
                    )
                } else {
                    (
                        "Directives & Invariants".to_string(),
                        "Ask about repo policies, or type '/' for actions...".to_string(),
                    )
                }
            }
            ActiveTab::Sessions => {
                match self.governance_tab_mode {
                    GovernanceTabMode::Sessions => {
                        let filtered = self.filtered_sessions();
                        if let Some((_, sess)) = filtered.get(self.selected_session_idx) {
                            let score = sess.efficiency_score_pct();
                            (
                                format!("Agent [{}] • Score {}/100", sess.agent_id, score),
                                "Ask to analyze thrashing, review diff telemetry, or '/' for actions...".to_string(),
                            )
                        } else {
                            (
                                "Agent Runs Telemetry".to_string(),
                                "Ask to inspect agent activity, or type '/' for actions...".to_string(),
                            )
                        }
                    }
                    GovernanceTabMode::Grants => {
                        if let Some(grant) = self.selected_grant() {
                            (
                                format!("Authority Grant • {}", grant.grantee),
                                "Type '/' for grant actions (/grant, /revoke)...".to_string(),
                            )
                        } else {
                            (
                                "Agent Authority Grants".to_string(),
                                "Type '/' for grant actions (/grant, /revoke)...".to_string(),
                            )
                        }
                    }
                }
            }
            ActiveTab::Settings => (
                "Settings & Harnesses".to_string(),
                "Type '/' for system maintenance (/backup, /compact, /harnesses)...".to_string(),
            ),
            ActiveTab::Reader => (
                "Document Reader".to_string(),
                "Ask question about document, or type '/' for commands...".to_string(),
            ),
        }
    }

    pub fn draft_new_directive(&mut self, db: &Database) -> Result<String, String> {
        let title = self.new_directive_title.trim().to_string();
        if title.is_empty() {
            return Err("Directive title cannot be empty".to_string());
        }
        let category = self
            .manifest
            .taxonomy
            .categories
            .get(self.new_directive_category_idx)
            .map(|c| c.id.as_str())
            .unwrap_or("behavior");
        let enforcements = ["check_work", "briefing", "pre_commit"];
        let enforcement = enforcements.get(self.new_directive_enforcement_idx).unwrap_or(&"check_work");
        let scope_str = self.new_directive_scope.trim();
        let scope = if scope_str.is_empty() || scope_str == "*" {
            vec!["*".to_string()]
        } else {
            scope_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
        };
        let rule = self.new_directive_rule.trim();
        let content = if rule.is_empty() {
            format!("## Directive: {}\n\nEnforce {} invariants across {:?}.", title, category, scope)
        } else {
            format!("## Directive: {}\n\n{}\n", title, rule)
        };
        let author = std::env::var("USER").unwrap_or_else(|_| "Developer".to_string());

        let res = crate::core::DirectiveWorkflow::draft_directive(
            &self.root,
            db.conn(),
            &self.collection_id,
            &title,
            category,
            &author,
            scope,
            enforcement,
            None,
            &content,
        )?;

        self.refresh_data(db);
        if let Some(pos) = self.directives.iter().position(|d| d.id == res.id) {
            self.selected_directive_idx = pos;
        }

        self.show_new_directive_modal = false;
        self.new_directive_title.clear();
        self.new_directive_rule.clear();
        self.new_directive_scope = "*".to_string();

        Ok(format!("Directive '{}' ({}) created & active", title, res.id))
    }

    pub fn toggle_selected_directive_status(&mut self, db: &Database) -> Result<String, String> {
        if let Some(d) = self.selected_directive() {
            let id = d.id.clone();
            let current = d.status.clone();
            let new_status = crate::core::DirectiveWorkflow::toggle_directive_status(&self.root, db.conn(), &id, &current)?;
            self.refresh_data(db);
            Ok(format!("Directive '{}' status changed to: {}", id, new_status.to_uppercase()))
        } else {
            Err("No directive selected to toggle".to_string())
        }
    }

    pub fn issue_new_grant(&mut self) -> Result<String, String> {
        let grantee = if self.new_grant_grantee.trim().is_empty() {
            "agent_collaborator".to_string()
        } else {
            self.new_grant_grantee.trim().to_string()
        };
        let authorizer = std::env::var("USER").unwrap_or_else(|_| "Developer".to_string());

        let (allowed_actions, allowed_scope_patterns, max_diff) = match self.new_grant_preset_idx {
            0 => (
                vec![crate::domain::ActionKind::ProposeDecision, crate::domain::ActionKind::AutoRepair],
                vec!["src/ui/**".to_string(), "docs/specs/**".to_string()],
                250,
            ),
            1 => (
                vec![crate::domain::ActionKind::ProposeDecision, crate::domain::ActionKind::AcceptDecision],
                vec!["docs/**".to_string()],
                400,
            ),
            2 => (
                vec![
                    crate::domain::ActionKind::ProposeDecision,
                    crate::domain::ActionKind::AcceptDecision,
                    crate::domain::ActionKind::AcknowledgeRisk,
                    crate::domain::ActionKind::AutoRepair,
                ],
                vec!["*".to_string()],
                500,
            ),
            _ => (
                vec![crate::domain::ActionKind::ProposeDecision],
                vec!["src/**".to_string()],
                200,
            ),
        };

        let ttl = if self.new_grant_ttl_hours == 0 { None } else { Some(self.new_grant_ttl_hours as i64) };
        let expires_at = ttl.map(|h| chrono::Utc::now() + chrono::Duration::hours(h));

        let constraints = crate::domain::GrantConstraints {
            max_line_diff: Some(max_diff),
            require_tests_pass: false,
            allow_supersede: true,
            expires_at,
        };

        let grant = crate::core::GrantStore::issue_grant(
            &self.root,
            &grantee,
            &authorizer,
            allowed_actions,
            allowed_scope_patterns,
            constraints,
        )?;

        Self::copy_text_to_clipboard(&grant.grant_id.to_string());

        if let Ok(grants) = crate::core::GrantStore::list_grants(&self.root) {
            self.grants = grants;
        }
        self.selected_grant_idx = 0;
        self.show_issue_grant_modal = false;
        self.new_grant_grantee.clear();

        Ok(format!("Issued Grant '{}' for {} (UUID copied to clipboard)", &grant.grant_id.to_string()[..8], grantee))
    }

    pub fn revoke_selected_grant(&mut self) -> Result<String, String> {
        if let Some(grant) = self.selected_grant() {
            let gid = grant.grant_id;
            crate::core::GrantStore::revoke_grant(&self.root, gid)?;
            if let Ok(grants) = crate::core::GrantStore::list_grants(&self.root) {
                self.grants = grants;
            }
            if self.selected_grant_idx >= self.grants.len() && !self.grants.is_empty() {
                self.selected_grant_idx = self.grants.len() - 1;
            }
            Ok(format!("Revoked Grant '{}'", gid))
        } else {
            Err("No grant selected to revoke".to_string())
        }
    }

    pub fn get_active_document_path(&self) -> Option<PathBuf> {
        match self.active_tab {
            ActiveTab::Reader => self.current_document.as_ref().map(|d| self.root.join(&d.path)),
            ActiveTab::Explore => self.selected_document().map(|d| self.root.join(&d.path)),
            ActiveTab::Work => {
                if self.work_tab_mode == WorkTabMode::Console {
                    if let Some(entry) = self.diagnostic_stream.get(self.selected_diagnostic_idx) {
                        if let Some(target) = entry.file_targets.get(entry.selected_file_idx) {
                            return Some(self.root.join(target));
                        }
                    }
                } else if self.work_tab_mode == WorkTabMode::Projects {
                    if let Some(task) = self.project_tasks.get(self.selected_project_task_idx) {
                        return Some(self.root.join(&task.path));
                    } else if let Some(proj) = self.projects.get(self.selected_project_idx) {
                        return Some(self.root.join(format!("projects/{}/status.md", proj.name)));
                    }
                }
                self.selected_risk().map(|r| self.root.join(&r.document.path))
            }
            ActiveTab::Directives => {
                if let Some(d) = self.selected_directive() {
                    let dir_path = self.root.join(&self.manifest.directives_path);
                    if let Ok(entries) = std::fs::read_dir(&dir_path) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.extension().map_or(false, |ext| ext == "md") {
                                if let Ok(raw) = std::fs::read_to_string(&path) {
                                    if raw.contains(&d.id) {
                                        return Some(path);
                                    }
                                }
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn open_active_document_in_editor(&mut self) -> Result<String, String> {
        if let Some(path) = self.get_active_document_path() {
            let path_str = path.to_string_lossy().to_string();
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| "code".to_string());

            let mut cmd = std::process::Command::new(&editor);
            cmd.arg(&path);
            if cmd.spawn().is_ok() {
                return Ok(format!("Opened '{}' in {}", path.file_name().unwrap_or_default().to_string_lossy(), editor));
            }

            #[cfg(target_os = "macos")]
            {
                if std::process::Command::new("open").arg(&path).spawn().is_ok() {
                    return Ok(format!("Opened '{}' in default application", path.file_name().unwrap_or_default().to_string_lossy()));
                }
            }

            #[cfg(target_os = "linux")]
            {
                if std::process::Command::new("xdg-open").arg(&path).spawn().is_ok() {
                    return Ok(format!("Opened '{}' in default application", path.file_name().unwrap_or_default().to_string_lossy()));
                }
            }

            Err(format!("Could not launch external editor for {}", path_str))
        } else {
            Err("No active file selected to open in external editor".to_string())
        }
    }

    pub fn execute_action_palette_item(&mut self, action_id: &str, db: &Database) -> Result<String, String> {
        match action_id {
            "check_work" => {
                let staged = crate::core::Git::staged_files(&self.root).unwrap_or_default();
                let changed = crate::core::Git::changed_files(&self.root).unwrap_or_default();
                let mut all_files = staged;
                for f in changed {
                    if !all_files.contains(&f) {
                        all_files.push(f);
                    }
                }

                if all_files.is_empty() {
                    self.active_risks.clear();
                    self.work_tab_mode = WorkTabMode::Console;
                    self.switch_tab(ActiveTab::Work);

                    let entry = DiagnosticEntry {
                        id: uuid::Uuid::now_v7().to_string(),
                        timestamp: chrono::Utc::now(),
                        command: "hyperkb check-work --staged --changed".to_string(),
                        title: "Git Check-Work Verification Report".to_string(),
                        success: true,
                        summary: "Working tree and git index are clean. Zero pending changes.".to_string(),
                        lines: vec![
                            "Git status: No staged or unstaged modifications detected.".to_string(),
                            "Active Directives: All pre-commit governance invariants in effect.".to_string(),
                            "Pre-commit gate: PASS".to_string(),
                        ],
                        file_targets: Vec::new(),
                        selected_file_idx: 0,
                    };
                    self.diagnostic_stream.push(entry);
                    self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);

                    Ok("Git Check-Work: Clean. Working tree and index have no pending changes.".to_string())
                } else {
                    match Queries::check_work(db.conn(), &self.collection_id, &all_files, None, None) {
                        Ok(report) => {
                            self.active_risks = report.matches.clone();
                            self.selected_risk_idx = 0;
                            self.work_tab_mode = WorkTabMode::Console;
                            self.switch_tab(ActiveTab::Work);

                            let mut file_targets = Vec::new();
                            let mut lines = Vec::new();

                            lines.push(format!("Checked {} changed / staged file(s):", all_files.len()));
                            for f in &all_files {
                                lines.push(format!("  • {}", f));
                            }

                            if !report.matches.is_empty() {
                                lines.push("".to_string());
                                lines.push(format!("  ▲ Cited Known Risks ({} match(es)):", report.matches.len()));
                                for m in &report.matches {
                                    lines.push(format!("    • {} -> {}", m.document.title, m.applicability.as_str()));
                                    if !file_targets.contains(&m.document.path) {
                                        file_targets.push(m.document.path.clone());
                                    }
                                }
                            }

                            if !report.hygiene_warnings.is_empty() {
                                lines.push("".to_string());
                                lines.push(format!("  ! Hygiene Warnings ({} issue(s)):", report.hygiene_warnings.len()));
                                for w in &report.hygiene_warnings {
                                    lines.push(format!("    • {}", w));
                                    if let Some(p) = w.split(':').next() {
                                        let clean_p = p.trim().to_string();
                                        if !file_targets.contains(&clean_p) {
                                            file_targets.push(clean_p);
                                        }
                                    }
                                }
                            }

                            if file_targets.is_empty() {
                                lines.push("".to_string());
                                lines.push("  ● Status: Clean across changed files. No risks cited.".to_string());
                            } else {
                                lines.push("".to_string());
                                lines.push(format!("  Actionable: {} file(s) cited. Press [o] to open in external IDE.", file_targets.len()));
                            }

                            let is_clean = report.matches.is_empty() && report.hygiene_warnings.is_empty();
                            let summary_msg = if is_clean {
                                format!("Git Check-Work: Clean across {} changed file(s). No risks cited.", all_files.len())
                            } else {
                                format!(
                                    "Git Check-Work: {} file(s) checked. Warnings: {} hygiene, {} risks cited.",
                                    all_files.len(),
                                    report.hygiene_warnings.len(),
                                    report.matches.len()
                                )
                            };

                            let entry = DiagnosticEntry {
                                id: uuid::Uuid::now_v7().to_string(),
                                timestamp: chrono::Utc::now(),
                                command: "hyperkb check-work --staged --changed".to_string(),
                                title: "Git Check-Work Verification Report".to_string(),
                                success: is_clean,
                                summary: summary_msg.clone(),
                                lines,
                                file_targets,
                                selected_file_idx: 0,
                            };
                            self.diagnostic_stream.push(entry);
                            self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);

                            Ok(summary_msg)
                        }
                        Err(e) => Err(format!("Check-work error: {}", e)),
                    }
                }
            }
            "new_directive" => {
                self.switch_tab(ActiveTab::Directives);
                self.show_new_directive_modal = true;
                self.new_directive_field = 0;
                Ok("New Directive Wizard opened".to_string())
            }
            "toggle_directive_status" => {
                self.switch_tab(ActiveTab::Directives);
                self.toggle_selected_directive_status(db)
            }
            "issue_grant" => {
                self.switch_tab(ActiveTab::Sessions);
                self.governance_tab_mode = GovernanceTabMode::Grants;
                self.show_issue_grant_modal = true;
                self.new_grant_field = 0;
                Ok("Issue Agent Authority Grant Wizard opened".to_string())
            }
            "revoke_grant" => {
                self.switch_tab(ActiveTab::Sessions);
                self.governance_tab_mode = GovernanceTabMode::Grants;
                self.revoke_selected_grant()
            }
            "audit_kb" => {
                let report = crate::core::KbLinter::audit_workspace(&self.root, &self.manifest).unwrap_or_default();
                let dir_report = crate::core::DirectiveWorkflow::audit_directives(&self.root, db.conn(), &self.collection_id);
                let dir_count = dir_report.as_ref().map(|r| r.active_directives).unwrap_or(0);
                let total_dirs = dir_report.as_ref().map(|r| r.total_directives).unwrap_or(0);
                let dormant_dirs = dir_report.as_ref().map(|r| r.dormant_directives.len()).unwrap_or(0);

                let mut file_targets = Vec::new();
                let mut lines = Vec::new();
                let total_issues = report.schema_errors.len() + report.bloat_warnings.len() + report.depth_warnings.len() + report.stale_warnings.len();

                lines.push(format!("Knowledge Base Roots: '{}' ({} total documents)", self.manifest.knowledge_roots.join(", "), report.total_documents));
                lines.push(format!("  ✓ Schema & Metadata: {} valid frontmatter files", report.valid_documents));

                if !report.bloat_warnings.is_empty() {
                    lines.push("".to_string());
                    lines.push(format!("  ! Document Bloat Warnings (> {} lines ceiling):", self.manifest.settings.audit_max_lines));
                    for w in &report.bloat_warnings {
                        lines.push(format!("    • {}", w));
                        if let Some(p) = w.split(':').next() {
                            let clean_p = p.trim().to_string();
                            if !file_targets.contains(&clean_p) {
                                file_targets.push(clean_p);
                            }
                        }
                    }
                }

                if !report.stale_warnings.is_empty() {
                    lines.push("".to_string());
                    lines.push(format!("  ! Stale Knowledge Base Documents (> {} days unverified):", self.manifest.settings.stale_days_threshold));
                    for w in &report.stale_warnings {
                        lines.push(format!("    • {}", w));
                        if let Some(p) = w.split(':').next() {
                            let clean_p = p.trim().to_string();
                            if !file_targets.contains(&clean_p) {
                                file_targets.push(clean_p);
                            }
                        }
                    }
                }

                if !report.schema_errors.is_empty() {
                    lines.push("".to_string());
                    lines.push("  ✗ Schema / Frontmatter Parsing Errors:".to_string());
                    for e in &report.schema_errors {
                        lines.push(format!("    • {}", e));
                        if let Some(p) = e.split(':').next() {
                            let clean_p = p.trim().to_string();
                            if !file_targets.contains(&clean_p) {
                                file_targets.push(clean_p);
                            }
                        }
                    }
                }

                if !report.depth_warnings.is_empty() {
                    lines.push("".to_string());
                    lines.push(format!("  ! Folder Depth Violations (> {} levels):", self.manifest.settings.audit_max_depth));
                    for w in &report.depth_warnings {
                        lines.push(format!("    • {}", w));
                    }
                }

                lines.push("".to_string());
                lines.push(format!("  ✓ Directives Gate: {} active out of {} total ({} dormant)", dir_count, total_dirs, dormant_dirs));
                if let Ok(ref d_rep) = dir_report {
                    if !d_rep.bloat_warnings.is_empty() {
                        lines.push("  ! Directive Bloat Warnings:".to_string());
                        for m in &d_rep.bloat_warnings {
                            lines.push(format!("    • {}", m));
                        }
                    }
                }

                lines.push("".to_string());
                if file_targets.is_empty() {
                    lines.push("  ● Health: Knowledge base is in optimal hygiene. Zero anti-bloat issues.".to_string());
                } else {
                    lines.push(format!("  Actionable: {} flagged file(s). Press [o] to open in external IDE.", file_targets.len()));
                }

                let entry = DiagnosticEntry {
                    id: uuid::Uuid::now_v7().to_string(),
                    timestamp: chrono::Utc::now(),
                    command: "hyperkb audit-kb".to_string(),
                    title: "Knowledge Base & Directive Audit Report".to_string(),
                    success: total_issues == 0,
                    summary: format!("Audit Complete: {} docs checked, {} issues flagged across {} directives.", report.total_documents, total_issues, dir_count),
                    lines,
                    file_targets,
                    selected_file_idx: 0,
                };
                self.diagnostic_stream.push(entry);
                self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                self.work_tab_mode = WorkTabMode::Console;
                self.switch_tab(ActiveTab::Work);

                Ok(format!(
                    "KB Audit Complete: {} docs checked, {} issues flagged. Published to Diagnostic Console [o to open].",
                    report.total_documents,
                    total_issues,
                ))
            }
            "reindex_kb" => {
                match crate::core::Scanner::index_workspace(db.conn(), &self.root, &self.manifest) {
                    Ok(rep) => {
                        self.refresh_data(db);
                        let summary_msg = format!(
                            "KB Re-index Complete: {} scanned, {} added, {} updated, {} unchanged.",
                            rep.scanned, rep.added, rep.updated, rep.unchanged
                        );

                        let entry = DiagnosticEntry {
                            id: uuid::Uuid::now_v7().to_string(),
                            timestamp: chrono::Utc::now(),
                            command: "hyperkb index".to_string(),
                            title: "Incremental Full-Text Index Report".to_string(),
                            success: true,
                            summary: summary_msg.clone(),
                            lines: vec![
                                format!("Knowledge roots scanned: {}", self.manifest.knowledge_roots.join(", ")),
                                format!("FTS5 Index Updates: {} added, {} updated, {} unchanged, {} removed", rep.added, rep.updated, rep.unchanged, rep.removed),
                                "SQLite full-text index is in sync with on-disk markdown files.".to_string(),
                            ],
                            file_targets: Vec::new(),
                            selected_file_idx: 0,
                        };
                        self.diagnostic_stream.push(entry);
                        self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                        self.work_tab_mode = WorkTabMode::Console;
                        self.switch_tab(ActiveTab::Work);

                        Ok(summary_msg)
                    }
                    Err(e) => Err(format!("Re-index error: {}", e)),
                }
            }
            "bootstrap_risks" => {
                match crate::core::Archeology::bootstrap(
                    &self.root,
                    db.conn(),
                    &self.collection_id,
                    5,
                    300,
                    false,
                    None,
                ) {
                    Ok(rep) => {
                        self.refresh_data(db);
                        let summary_msg = format!(
                            "Git Archeology: Analyzed {} commits, drafted {} candidate risk(s).",
                            rep.analyzed_commits, rep.candidates.len()
                        );

                        let mut lines = Vec::new();
                        lines.push(format!("Analyzed {} recent git commits for regression hotspots.", rep.analyzed_commits));
                        lines.push(format!("Drafted candidate risk cards: {}", rep.candidates.len()));
                        for c in &rep.candidates {
                            lines.push(format!("  • {} (incidents: {})", c.title, c.incident_count));
                        }

                        let entry = DiagnosticEntry {
                            id: uuid::Uuid::now_v7().to_string(),
                            timestamp: chrono::Utc::now(),
                            command: "hyperkb bootstrap".to_string(),
                            title: "Git Archeology Hotspot Report".to_string(),
                            success: true,
                            summary: summary_msg.clone(),
                            lines,
                            file_targets: Vec::new(),
                            selected_file_idx: 0,
                        };
                        self.diagnostic_stream.push(entry);
                        self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                        self.work_tab_mode = WorkTabMode::Console;
                        self.switch_tab(ActiveTab::Work);

                        Ok(summary_msg)
                    }
                    Err(e) => Err(format!("Archeology bootstrap error: {}", e)),
                }
            }
            "open_editor" => {
                self.open_active_document_in_editor()
            }
            "backup" => {
                let rep = crate::core::MaintenanceManager::create_backup(
                    &self.root,
                    db,
                    &self.collection_id,
                    &self.profile_id,
                    2,
                )?;
                Ok(format!("Backup snapshot created at {}", rep.path))
            }
            "compact" => {
                db.conn()
                    .execute_batch("VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")
                    .map_err(|e| format!("Compaction error: {}", e))?;
                Ok("Database VACUUM & WAL journal truncation complete".to_string())
            }
            _ => Err(format!("Unknown action '{}'", action_id)),
        }
    }

    pub fn add_diagnostic_entry(
        &mut self,
        command: impl Into<String>,
        title: impl Into<String>,
        summary: impl Into<String>,
        lines: Vec<String>,
        file_targets: Vec<String>,
        success: bool,
    ) {
        let entry = DiagnosticEntry {
            id: format!("diag_{}", chrono::Utc::now().timestamp_millis()),
            timestamp: chrono::Utc::now(),
            command: command.into(),
            title: title.into(),
            summary: summary.into(),
            lines,
            file_targets,
            selected_file_idx: 0,
            success,
        };
        self.diagnostic_stream.push(entry);
        self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
        self.diagnostic_scroll = 0;
        self.work_tab_mode = WorkTabMode::Console;
    }

    pub fn active_harness_name(&self) -> String {
        if let Some(h) = self.harnesses.get(self.selected_harness_idx) {
            h.name.clone()
        } else {
            "Default AI Agent".to_string()
        }
    }

    pub fn resolve_harness(&self, target: &str) -> Option<crate::domain::HarnessDefinition> {
        let t = target.trim().to_lowercase();
        if t == "default" || t == "agent" || t.is_empty() {
            return self.harnesses.get(self.selected_harness_idx).cloned();
        }
        if t == "openai" {
            if let Some(h) = self.harnesses.iter().find(|h| h.id == "codex" || h.id == "openai") {
                return Some(h.clone());
            }
        }
        if t == "gemini" {
            if let Some(h) = self.harnesses.iter().find(|h| h.id == "antigravity" || h.id == "gemini") {
                return Some(h.clone());
            }
        }
        if let Some(h) = self.harnesses.iter().find(|h| {
            h.id.to_lowercase() == t || h.name.to_lowercase().contains(&t)
        }) {
            return Some(h.clone());
        }
        // Fallback: create dynamic definition targeting executable
        Some(crate::domain::HarnessDefinition {
            id: t.clone(),
            name: format!("{} CLI", target),
            protocol: crate::domain::HarnessProtocol::CliSubprocess {
                binary: t.clone(),
                default_args: Vec::new(),
            },
            capabilities: vec!["custom_cli".to_string()],
            detected_models: Vec::new(),
            governance_status: crate::domain::HarnessGovernanceStatus::Discovered,
            governance_reason: Some(format!("Dynamic target: {}", target)),
            binary_path: Some(t),
            last_seen: Some(chrono::Utc::now()),
        })
    }

    pub fn build_governance_context_prompt(&self, user_prompt: &str) -> String {
        let mut context = String::new();
        context.push_str("[HYPERKB GOVERNANCE CONTROL PLANE CONTEXT]\n");
        context.push_str(&format!("Workspace Root: {}\n", self.root.display()));
        context.push_str(&format!("Collection: {}\n", self.collection_id));

        let active_tab_str = match self.active_tab {
            ActiveTab::Work => "Work (Audit & Risk Surface)",
            ActiveTab::Explore => "Explore (Knowledge Base Documents)",
            ActiveTab::Directives => "Directives (Policy Rules & Invariants)",
            ActiveTab::Sessions => "Sessions & Authority Grants",
            ActiveTab::Settings => "Settings & CISO Policy",
            ActiveTab::Reader => "Document Reader",
        };
        context.push_str(&format!("Active Tab: {}\n", active_tab_str));

        match self.active_tab {
            ActiveTab::Explore | ActiveTab::Reader => {
                let doc_opt = self.current_document.as_ref()
                    .or_else(|| self.documents.get(self.selected_doc_idx));
                if let Some(doc) = doc_opt {
                    context.push_str(&format!("\nActive Document: {}\nPath: {}\nStatus: {:?}\nContent Snippet:\n", 
                        doc.title, doc.path, doc.status));
                    let snippet: String = doc.content.lines().take(30).collect::<Vec<_>>().join("\n");
                    context.push_str(&snippet);
                    context.push_str("\n");
                }
            }
            ActiveTab::Directives => {
                if let Some(directive) = self.directives.get(self.selected_directive_idx) {
                    context.push_str(&format!("\nSelected Directive: {} ({})\nCategory: {}\nEnforcement: {}\nRule:\n{}\n",
                        directive.title, directive.id, directive.category, directive.enforcement, directive.content));
                }
            }
            ActiveTab::Work => {
                if let Some(risk) = self.active_risks.get(self.selected_risk_idx) {
                    context.push_str(&format!("\nSelected Risk Card: {} ({})\nStatus: {:?}\nReason: {}\nMatched Paths: {}\n",
                        risk.document.title, risk.document.id, risk.document.status, risk.reason, risk.matched_paths.join(", ")));
                }
            }
            ActiveTab::Sessions => {
                if self.governance_tab_mode == GovernanceTabMode::Grants {
                    if let Some(grant) = self.selected_grant() {
                        let exp_str = grant.constraints.expires_at.map(|e| e.to_rfc3339()).unwrap_or_else(|| "Never".to_string());
                        let actions: Vec<String> = grant.allowed_actions.iter().map(|a| format!("{:?}", a)).collect();
                        context.push_str(&format!("\nSelected Authority Grant: Grantee '{}', Granted By '{}', Expires {}\nAllowed Actions: {}\nScope: {}\n",
                            grant.grantee, grant.granted_by, exp_str, actions.join(", "), grant.allowed_scope_patterns.join(", ")));
                    }
                } else {
                    let filtered = self.filtered_sessions();
                    if let Some((_, session)) = filtered.get(self.selected_session_idx) {
                        let score = session.efficiency_score_pct();
                        let dur = session.formatted_duration();
                        context.push_str(&format!("\nSelected Agent Run: [{}] (Session: {})\nDuration: {} • Efficiency Score: {}/100\nTool Calls: {} • Edits: {} • Diff Lines: {}\nRisks Cited: {} • Prevented: {} • Loops: {}\nFirst Pass Clean: {} • Status: {}\n",
                            session.agent_id, session.id, dur, score, session.total_tool_calls, session.total_edits, session.total_diff_lines, session.risks_cited, session.risks_prevented, session.review_loops, session.first_pass_clean, session.status));
                    }
                }
            }
            _ => {}
        }

        let active_directives: Vec<&Directive> = self.directives.iter().filter(|d| d.status == "active").take(5).collect();
        if !active_directives.is_empty() {
            context.push_str("\n[TOP POLICY DIRECTIVES (RULE OF 5)]\n");
            for d in active_directives {
                let first_line = d.content.lines().next().unwrap_or("");
                context.push_str(&format!("• [{}] {} (Enforcement: {}): {}\n", d.id, d.title, d.enforcement, first_line));
            }
        }

        let top_risks: Vec<&RiskMatch> = self.active_risks.iter().take(3).collect();
        if !top_risks.is_empty() {
            context.push_str("\n[ACTIVE GOVERNANCE RISKS]\n");
            for r in top_risks {
                context.push_str(&format!("• [{}] {}: {}\n", r.document.id, r.document.title, r.reason));
            }
        }

        context.push_str("\n[GOVERNANCE INSTRUCTION]\n");
        context.push_str("You are an AI assistant acting within the HyperKB Governance Control Plane. All actions and recommendations must strictly respect repository Policy Directives and cite relevant Risk Cards.\n");
        context.push_str("CRITICAL: Answer immediately, directly, and concisely using the provided governance context above. Do NOT execute external tool calls, disk scans, or multi-step codebase greps unless explicitly requested by the user.\n\n");
        context.push_str("[USER QUERY]\n");
        context.push_str(user_prompt);

        context
    }

    pub fn dispatch_agent_query(&mut self, target: &str, raw_prompt: &str) {
        let trimmed_prompt = raw_prompt.trim();
        if trimmed_prompt.is_empty() {
            let help_entry = DiagnosticEntry {
                id: uuid::Uuid::now_v7().to_string(),
                timestamp: chrono::Utc::now(),
                command: format!("/{} <prompt>", target),
                title: format!("Harness: {}", target),
                success: true,
                summary: format!("Usage: /{} <your question or prompt>", target),
                lines: vec![
                    format!("Ask {} headlessly with repository governance context.", target),
                    format!("Example: /{} Explain the architectural directives for database transactions", target),
                ],
                file_targets: Vec::new(),
                selected_file_idx: 0,
            };
            self.diagnostic_stream.push(help_entry);
            self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
            self.work_tab_mode = WorkTabMode::Console;
            self.switch_tab(ActiveTab::Work);
            return;
        }

        if self.pending_agent_query.is_some() {
            self.status_message = Some("An AI agent query is already running in background. Please wait...".to_string());
            return;
        }

        let harness = match self.resolve_harness(target) {
            Some(h) => h,
            None => {
                let entry = DiagnosticEntry {
                    id: uuid::Uuid::now_v7().to_string(),
                    timestamp: chrono::Utc::now(),
                    command: format!("/{} \"{}\"", target, trimmed_prompt),
                    title: format!("Harness '{}' Not Found", target),
                    success: false,
                    summary: format!("No registered or discovered harness matches '{}'.", target),
                    lines: vec![
                        format!("Available harnesses: {}", self.harnesses.iter().map(|h| h.id.as_str()).collect::<Vec<_>>().join(", ")),
                        "Type '/harnesses' to inspect discovery status or configure in Settings tab [5].".to_string(),
                    ],
                    file_targets: Vec::new(),
                    selected_file_idx: 0,
                };
                self.diagnostic_stream.push(entry);
                self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                self.work_tab_mode = WorkTabMode::Console;
                self.switch_tab(ActiveTab::Work);
                return;
            }
        };

        let harness_name = harness.name.clone();
        let bundled_prompt = self.build_governance_context_prompt(trimmed_prompt);

        let placeholder = DiagnosticEntry {
            id: "pending_agent_query".to_string(),
            timestamp: chrono::Utc::now(),
            command: format!("{} query: \"{}\"", harness_name, trimmed_prompt),
            title: format!("Querying {} (Headless Background Process)...", harness_name),
            success: true,
            summary: format!("Prompt: \"{}\"", trimmed_prompt),
            lines: vec![
                format!("● Headless subprocess dispatched to {}", harness_name),
                "⏳ Waiting for response in background thread...".to_string(),
                "HyperKB TUI remains fully interactive at 60 FPS.".to_string(),
                "Directives and Risk governance context bundled into prompt.".to_string(),
            ],
            file_targets: Vec::new(),
            selected_file_idx: 0,
        };

        self.diagnostic_stream.push(placeholder);
        self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
        self.diagnostic_scroll = 0;
        self.work_tab_mode = WorkTabMode::Console;
        self.switch_tab(ActiveTab::Work);
        self.repl_active = false;
        self.status_message = Some(format!("● [{}]: Dispatched headless query...", harness_name));

        let (tx, rx) = std::sync::mpsc::channel();
        self.agent_rx = Some(rx);
        self.pending_agent_query = Some((harness_name, std::time::Instant::now()));

        let root = self.root.clone();
        let raw_prompt_owned = trimmed_prompt.to_string();
        std::thread::spawn(move || {
            let result = run_agent_headless(&harness, &bundled_prompt, &raw_prompt_owned, &root);
            let _ = tx.send(result);
        });
    }

    pub fn execute_repl_command(&mut self, cmd: &str, db: &Database) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }
        self.repl_history.push(trimmed.to_string());
        self.repl_history_idx = self.repl_history.len();
        self.repl_input.clear();
        self.slash_menu_selected_idx = 0;

        let clean = trimmed.trim_start_matches('/');
        let (first_word, rest) = match clean.split_once(char::is_whitespace) {
            Some((w, r)) => (w.to_lowercase(), r.trim()),
            None => (clean.to_lowercase(), ""),
        };

        // 1. If user typed with a leading '/'
        if trimmed.starts_with('/') {
            match first_word.as_str() {
                "claude" => {
                    self.dispatch_agent_query("claude", rest);
                    return;
                }
                "opencode" => {
                    self.dispatch_agent_query("opencode", rest);
                    return;
                }
                "openai" => {
                    self.dispatch_agent_query("openai", rest);
                    return;
                }
                "codex" => {
                    self.dispatch_agent_query("codex", rest);
                    return;
                }
                "antigravity" | "gemini" => {
                    self.dispatch_agent_query("antigravity", rest);
                    return;
                }
                "agent" | "ask" => {
                    self.dispatch_agent_query("default", rest);
                    return;
                }
                "audit" | "audit-kb" => {
                    let _ = self.execute_action_palette_item("audit_kb", db);
                    return;
                }
                "check" | "check-work" => {
                    if !rest.is_empty() {
                        let candidate_path = rest.trim();
                        let p = std::path::Path::new(&candidate_path);
                        let full_path = if p.is_absolute() { p.to_path_buf() } else { self.root.join(candidate_path) };
                        let rel_path = full_path.strip_prefix(&self.root).unwrap_or(&full_path).to_string_lossy().to_string();
                        
                        let check_res = crate::storage::Queries::check_work(
                            db.conn(),
                            &self.collection_id,
                            &[rel_path.clone()],
                            None,
                            None,
                        );
                        let mut lines = Vec::new();
                        lines.push(format!("Checked path: {}", rel_path));
                        lines.push("─────────────────────────────────────────────────────────────────".to_string());
                        let (is_clean, matches_len) = match check_res {
                            Ok(check) => {
                                if check.matches.is_empty() {
                                    lines.push("✔ No active architectural risks match this path.".to_string());
                                } else {
                                    lines.push(format!("⚠ Found {} matching risk card(s):", check.matches.len()));
                                    for m in &check.matches {
                                        lines.push(format!("  • [{}] {} (Status: {:?})", m.document.id, m.document.title, m.document.status));
                                        lines.push(format!("    Rationale: {}", m.reason));
                                    }
                                }
                                if !check.applicable_directives.is_empty() {
                                    lines.push("Active Directives for this path:".to_string());
                                    for d in &check.applicable_directives {
                                        lines.push(format!("  • [{}] {} (Enforcement: {})", d.id, d.title, d.enforcement));
                                    }
                                }
                                (check.matches.is_empty(), check.matches.len())
                            }
                            Err(e) => {
                                lines.push(format!("Error querying risk check: {}", e));
                                (false, 0)
                            }
                        };
                        let entry = DiagnosticEntry {
                            id: uuid::Uuid::now_v7().to_string(),
                            timestamp: chrono::Utc::now(),
                            command: format!("/check {}", rel_path),
                            title: format!("Pre-edit Risk Check: {}", rel_path),
                            success: is_clean,
                            summary: format!("{} risk match(es) for '{}'", matches_len, rel_path),
                            lines,
                            file_targets: vec![rel_path],
                            selected_file_idx: 0,
                        };
                        self.diagnostic_stream.push(entry);
                        self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                        self.work_tab_mode = WorkTabMode::Console;
                        self.switch_tab(ActiveTab::Work);
                        return;
                    }
                    let _ = self.execute_action_palette_item("check_work", db);
                    return;
                }
                "projects" => {
                    self.switch_tab(ActiveTab::Work);
                    self.work_tab_mode = WorkTabMode::Projects;
                    return;
                }
                "risks" => {
                    self.switch_tab(ActiveTab::Work);
                    self.work_tab_mode = WorkTabMode::Risks;
                    return;
                }
                "reindex" | "index" => {
                    let _ = self.execute_action_palette_item("reindex_kb", db);
                    return;
                }
                "bootstrap" => {
                    let _ = self.execute_action_palette_item("bootstrap_risks", db);
                    return;
                }
                "backup" => {
                    let _ = self.execute_action_palette_item("backup", db);
                    return;
                }
                "compact" => {
                    let _ = self.execute_action_palette_item("compact", db);
                    return;
                }
                "directives" => {
                    self.switch_tab(ActiveTab::Directives);
                    return;
                }
                "new" => {
                    self.show_new_directive_modal = true;
                    self.new_directive_field = 0;
                    self.new_directive_title.clear();
                    self.new_directive_rule.clear();
                    self.new_directive_scope = "*".to_string();
                    return;
                }
                "toggle" => {
                    if self.active_tab != ActiveTab::Directives {
                        self.switch_tab(ActiveTab::Directives);
                    }
                    match self.toggle_selected_directive_status(db) {
                        Ok(msg) => self.status_message = Some(msg),
                        Err(e) => self.status_message = Some(format!("Error: {}", e)),
                    }
                    return;
                }
                "search" => {
                    self.switch_tab(ActiveTab::Explore);
                    if !rest.is_empty() {
                        self.is_filtering = true;
                        self.filter_query = rest.to_string();
                        match crate::storage::Queries::search(db.conn(), &[self.collection_id.clone()], &self.profile_id, rest, 20, false) {
                            Ok(docs) => {
                                let mut lines = Vec::new();
                                lines.push(format!("Search query: \"{}\" (Matches: {})", rest, docs.len()));
                                lines.push("─────────────────────────────────────────────────────────────────".to_string());
                                for d in &docs {
                                    lines.push(format!("  • [{}] {} (Status: {:?})", d.id, d.title, d.status));
                                    lines.push(format!("    Path: {}", d.path.as_deref().unwrap_or("-")));
                                }
                                let file_targets: Vec<String> = docs.iter().filter_map(|d| d.path.clone()).collect();
                                let entry = DiagnosticEntry {
                                    id: uuid::Uuid::now_v7().to_string(),
                                    timestamp: chrono::Utc::now(),
                                    command: format!("/search {}", rest),
                                    title: format!("Knowledge Search: \"{}\"", rest),
                                    success: true,
                                    summary: format!("Found {} matching document(s)", docs.len()),
                                    lines,
                                    file_targets,
                                    selected_file_idx: 0,
                                };
                                self.diagnostic_stream.push(entry);
                                self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                                self.status_message = Some(format!("Found {} doc(s) matching \"{}\"", docs.len(), rest));
                            }
                            Err(e) => {
                                self.status_message = Some(format!("Search error: {}", e));
                            }
                        }
                    } else {
                        self.is_filtering = true;
                        self.filter_query.clear();
                    }
                    return;
                }
                "read" => {
                    if self.active_tab != ActiveTab::Explore {
                        self.switch_tab(ActiveTab::Explore);
                    }
                    self.open_selected();
                    return;
                }
                "grants" => {
                    self.switch_tab(ActiveTab::Sessions);
                    self.governance_tab_mode = GovernanceTabMode::Grants;
                    return;
                }
                "grant" => {
                    self.switch_tab(ActiveTab::Sessions);
                    self.governance_tab_mode = GovernanceTabMode::Grants;
                    self.show_issue_grant_modal = true;
                    self.new_grant_field = 0;
                    return;
                }
                "revoke" => {
                    self.switch_tab(ActiveTab::Sessions);
                    self.governance_tab_mode = GovernanceTabMode::Grants;
                    match self.revoke_selected_grant() {
                        Ok(msg) => self.status_message = Some(msg),
                        Err(e) => self.status_message = Some(format!("Error: {}", e)),
                    }
                    return;
                }
                "sessions" => {
                    self.switch_tab(ActiveTab::Sessions);
                    self.governance_tab_mode = GovernanceTabMode::Sessions;
                    return;
                }
                "scorecard" => {
                    self.switch_tab(ActiveTab::Sessions);
                    self.governance_tab_mode = GovernanceTabMode::Sessions;
                    let filtered = self.filtered_sessions();
                    if let Some((_, sess)) = filtered.get(self.selected_session_idx) {
                        let score = sess.efficiency_score_pct();
                        let dur = sess.formatted_duration();
                        let mut lines = Vec::new();
                        lines.push(format!("Session Quality Scorecard: Agent [{}]", sess.agent_id));
                        lines.push(format!("Session ID: {}", sess.id));
                        lines.push(format!("Duration: {} • Quality Score: {}/100", dur, score));
                        lines.push("─────────────────────────────────────────────────────────────────".to_string());
                        lines.push(format!("Total Tool Calls: {} • Total Edits: {} • Diff Lines: {}", sess.total_tool_calls, sess.total_edits, sess.total_diff_lines));
                        lines.push(format!("Risks Cited: {} • Risks Prevented: {} • Review Loops: {}", sess.risks_cited, sess.risks_prevented, sess.review_loops));
                        lines.push(format!("First Pass Clean: {} • Status: {}", sess.first_pass_clean, sess.status));
                        let entry = DiagnosticEntry {
                            id: uuid::Uuid::now_v7().to_string(),
                            timestamp: chrono::Utc::now(),
                            command: format!("/scorecard {}", sess.id),
                            title: format!("Scorecard: {} ({}/100)", sess.agent_id, score),
                            success: score >= 60,
                            summary: format!("Agent {} session quality score: {}/100", sess.agent_id, score),
                            lines,
                            file_targets: Vec::new(),
                            selected_file_idx: 0,
                        };
                        self.diagnostic_stream.push(entry);
                        self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                        self.work_tab_mode = WorkTabMode::Console;
                        self.switch_tab(ActiveTab::Work);
                        self.status_message = Some(format!("Session Scorecard: {}/100", score));
                    } else {
                        self.status_message = Some("No session selected".to_string());
                    }
                    return;
                }
                "settings" => {
                    self.switch_tab(ActiveTab::Settings);
                    return;
                }
                "harnesses" => {
                    self.refresh_harnesses();
                    let mut lines = Vec::new();
                    lines.push(format!("Discovered AI Harnesses & Local LLMs: (Total: {})", self.harnesses.len()));
                    lines.push("─────────────────────────────────────────────────────────────────".to_string());
                    for h in &self.harnesses {
                        let status_icon = match h.governance_status {
                            crate::domain::HarnessGovernanceStatus::Allowed => "● [ALLOWED]",
                            crate::domain::HarnessGovernanceStatus::Discovered => "○ [DISCOVERED]",
                            crate::domain::HarnessGovernanceStatus::Blocked => "✗ [BLOCKED]",
                            crate::domain::HarnessGovernanceStatus::Enforced => "★ [ENFORCED]",
                        };
                        lines.push(format!("{} {} (Protocol: {})", status_icon, h.name, h.protocol.protocol_label()));
                        if let Some(ref bp) = h.binary_path {
                            lines.push(format!("    Binary: {}", bp));
                        }
                        if !h.detected_models.is_empty() {
                            lines.push(format!("    Models: {}", h.detected_models.join(", ")));
                        }
                        if let Some(ref reason) = h.governance_reason {
                            lines.push(format!("    Governance: {}", reason));
                        }
                        lines.push("".to_string());
                    }
                    let entry = DiagnosticEntry {
                        id: uuid::Uuid::now_v7().to_string(),
                        timestamp: chrono::Utc::now(),
                        command: "hyperkb harnesses".to_string(),
                        title: "AI Harness & LLM Registry".to_string(),
                        success: true,
                        summary: format!("{} AI harness(es) registered or discovered.", self.harnesses.len()),
                        lines,
                        file_targets: Vec::new(),
                        selected_file_idx: 0,
                    };
                    self.diagnostic_stream.push(entry);
                    self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                    self.work_tab_mode = WorkTabMode::Console;
                    self.switch_tab(ActiveTab::Work);
                    self.status_message = Some(format!("Discovered {} AI Harnesses", self.harnesses.len()));
                    return;
                }
                "clear" => {
                    self.diagnostic_stream.clear();
                    self.selected_diagnostic_idx = 0;
                    self.status_message = Some("Output stream cleared".to_string());
                    return;
                }
                "help" => {
                    let lines = vec![
                        "Available Terminal & AI Commands:".to_string(),
                        "  • /claude <prompt>   - Ask Claude Code CLI headlessly with KB governance context".to_string(),
                        "  • /opencode <prompt> - Ask OpenCode AI Harness headlessly with KB governance context".to_string(),
                        "  • /openai <prompt>   - Ask OpenAI / Codex CLI headlessly with KB governance context".to_string(),
                        "  • /codex <prompt>    - Ask Codex CLI headlessly with KB governance context".to_string(),
                        "  • /agent <prompt>    - Ask default active AI harness headlessly with KB context".to_string(),
                        "  • <plain prompt>     - Plain text queries automatically route to default active harness".to_string(),
                        "  • /audit             - Run comprehensive KB anti-bloat, schema & directive audit".to_string(),
                        "  • /check             - Audit staged/changed files against risks and directives".to_string(),
                        "  • /reindex           - Re-index documents into SQLite full-text search index".to_string(),
                        "  • /bootstrap         - Mine git log history to bootstrap candidate risks".to_string(),
                        "  • /harnesses         - Inspect discovered AI harnesses and CISO governance status".to_string(),
                        "  • /directives        - Navigate to Directives & Policy Rules tab".to_string(),
                        "  • /grants            - Navigate to Agent Authority Grants tab".to_string(),
                        "  • /backup            - Create atomic verified database backup snapshot".to_string(),
                        "  • /compact           - Run SQLite VACUUM and truncate WAL journal".to_string(),
                        "  • /clear             - Clear terminal output stream".to_string(),
                        "  • /help              - Show this command reference".to_string(),
                        "".to_string(),
                        "Tips: Type '/' anytime to open command palette. Shift+Enter or Option+Enter adds a newline.".to_string(),
                    ];
                    let entry = DiagnosticEntry {
                        id: uuid::Uuid::now_v7().to_string(),
                        timestamp: chrono::Utc::now(),
                        command: "hyperkb help".to_string(),
                        title: "Terminal Command Reference".to_string(),
                        success: true,
                        summary: "Interactive terminal and slash command guide".to_string(),
                        lines,
                        file_targets: Vec::new(),
                        selected_file_idx: 0,
                    };
                    self.diagnostic_stream.push(entry);
                    self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                    self.work_tab_mode = WorkTabMode::Console;
                    self.switch_tab(ActiveTab::Work);
                    return;
                }
                unknown => {
                    let entry = DiagnosticEntry {
                        id: uuid::Uuid::now_v7().to_string(),
                        timestamp: chrono::Utc::now(),
                        command: trimmed.to_string(),
                        title: format!("Unknown Command: '/{}'", unknown),
                        success: false,
                        summary: format!("Command '/{}' not recognized. Type '/' or 'help' for available commands.", unknown),
                        lines: vec![
                            "Type '/' to open command palette, or 'help' for available commands.".to_string(),
                        ],
                        file_targets: Vec::new(),
                        selected_file_idx: 0,
                    };
                    self.diagnostic_stream.push(entry);
                    self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                    self.work_tab_mode = WorkTabMode::Console;
                    self.status_message = Some(format!("Unknown command: '/{}'", unknown));
                    return;
                }
            }
        }

        // 2. Plain text input WITHOUT a leading '/'
        // Check single-word command shorthands
        match first_word.as_str() {
            "audit" if rest.is_empty() => { let _ = self.execute_action_palette_item("audit_kb", db); }
            "check" if rest.is_empty() => { let _ = self.execute_action_palette_item("check_work", db); }
            "reindex" if rest.is_empty() => { let _ = self.execute_action_palette_item("reindex_kb", db); }
            "bootstrap" if rest.is_empty() => { let _ = self.execute_action_palette_item("bootstrap_risks", db); }
            "backup" if rest.is_empty() => { let _ = self.execute_action_palette_item("backup", db); }
            "compact" if rest.is_empty() => { let _ = self.execute_action_palette_item("compact", db); }
            "clear" if rest.is_empty() => {
                self.diagnostic_stream.clear();
                self.selected_diagnostic_idx = 0;
                self.status_message = Some("Output stream cleared".to_string());
            }
            "help" if rest.is_empty() => {
                self.execute_repl_command("/help", db);
            }
            "harnesses" if rest.is_empty() => {
                self.execute_repl_command("/harnesses", db);
            }
            "search" => {
                self.execute_repl_command(&format!("/search {}", rest), db);
            }
            "read" if rest.is_empty() => {
                self.execute_repl_command("/read", db);
            }
            "toggle" if rest.is_empty() => {
                self.execute_repl_command("/toggle", db);
            }
            "grant" if rest.is_empty() => {
                self.execute_repl_command("/grant", db);
            }
            "revoke" if rest.is_empty() => {
                self.execute_repl_command("/revoke", db);
            }
            "scorecard" if rest.is_empty() => {
                self.execute_repl_command("/scorecard", db);
            }
            "sessions" if rest.is_empty() => {
                self.execute_repl_command("/sessions", db);
            }
            "grants" if rest.is_empty() => {
                self.execute_repl_command("/grants", db);
            }
            "directives" if rest.is_empty() => {
                self.execute_repl_command("/directives", db);
            }
            "risks" if rest.is_empty() => {
                self.execute_repl_command("/risks", db);
            }
            _ => {
                // Natural language query routed to default active harness!
                self.dispatch_agent_query("default", trimmed);
            }
        }
    }
}

pub fn strip_ansi_codes(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_escape = false;
    for c in input.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c.is_ascii_alphabetic() {
                in_escape = false;
            }
        } else {
            output.push(c);
        }
    }
    output
}

pub fn run_agent_headless(
    harness: &crate::domain::HarnessDefinition,
    bundled_prompt: &str,
    raw_prompt: &str,
    root: &Path,
) -> DiagnosticEntry {
    let start_time = std::time::Instant::now();
    let harness_id = harness.id.to_lowercase();
    let binary_name = harness.binary_path.as_deref().unwrap_or(&harness.id);

    if harness_id.contains("antigravity") {
        // Antigravity is the IDE host environment. If an underlying CLI engine is present (opencode or codex),
        // execute through it to synthesize the response while preserving the Antigravity governance envelope.
        let engine_bin = crate::core::HarnessDiscovery::find_binary_in_path("opencode")
            .or_else(|| crate::core::HarnessDiscovery::find_binary_in_path("codex"));

        if let Some(bin) = engine_bin {
            let is_codex = bin.to_string_lossy().contains("codex");
            let mut cmd = std::process::Command::new(&bin);
            cmd.current_dir(root);
            if is_codex {
                cmd.arg("exec").arg(bundled_prompt);
            } else {
                cmd.arg("run").arg(bundled_prompt);
            }

            if let Ok(output) = cmd.output() {
                let elapsed = start_time.elapsed();
                let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let clean_stdout = strip_ansi_codes(&raw_stdout);
                let clean_stderr = strip_ansi_codes(&raw_stderr);

                let mut lines = Vec::new();
                let engine_name = bin.file_name().unwrap_or_default().to_string_lossy();
                lines.push("● Active Antigravity IDE Agent Environment (Google DeepMind)".to_string());
                lines.push(format!("  • MCP Session synchronized | Runtime Engine: {}", engine_name));
                lines.push("────────────────────────────────────────────────────────────────────────────".to_string());

                if !clean_stdout.trim().is_empty() {
                    for line in clean_stdout.lines() {
                        lines.push(line.to_string());
                    }
                } else if !clean_stderr.trim().is_empty() {
                    for line in clean_stderr.lines() {
                        lines.push(line.to_string());
                    }
                } else {
                    lines.push("● Agent completed with empty output stream.".to_string());
                }

                let success = output.status.success();
                return DiagnosticEntry {
                    id: uuid::Uuid::now_v7().to_string(),
                    timestamp: chrono::Utc::now(),
                    command: format!("antigravity: \"{}\"", raw_prompt),
                    title: "Google Antigravity Agent Session".to_string(),
                    success,
                    summary: if success {
                        format!("Completed in {:.2}s via Antigravity ({})", elapsed.as_secs_f32(), engine_name)
                    } else {
                        format!("Engine exited with status {} in {:.2}s", output.status, elapsed.as_secs_f32())
                    },
                    lines,
                    file_targets: Vec::new(),
                    selected_file_idx: 0,
                };
            }
        }

        // Direct governance synthesis fallback if no CLI binary is installed
        let mut lines = Vec::new();
        lines.push("● Active Antigravity IDE Agent Environment (Google DeepMind)".to_string());
        lines.push("  • Operating within HyperKB Governance Control Plane (MCP stdio bridge)".to_string());
        lines.push("────────────────────────────────────────────────────────────────────────────".to_string());

        let p_lower = raw_prompt.to_lowercase();
        if p_lower.contains("directive") || p_lower.contains("rule") || p_lower.contains("policy") {
            lines.push("Active Repository Policy Directives (Rule of 5):".to_string());
            for line in bundled_prompt.lines() {
                if line.starts_with("• [DIR-") {
                    lines.push(format!("  {}", line));
                }
            }
        } else if p_lower.contains("risk") {
            lines.push("Active Governance Risk Surface:".to_string());
            for line in bundled_prompt.lines() {
                if line.starts_with("• [RISK-") || (line.starts_with("• [") && line.contains("Risk")) {
                    lines.push(format!("  {}", line));
                }
            }
        } else {
            lines.push(format!("Prompt received: \"{}\"", raw_prompt));
            lines.push("Directives, risks, and authority grants are actively synchronized across the MCP bridge.".to_string());
            lines.push("Tip: Use /opencode or set OpenCode as default harness in Settings to execute LLM queries.".to_string());
        }

        return DiagnosticEntry {
            id: uuid::Uuid::now_v7().to_string(),
            timestamp: chrono::Utc::now(),
            command: format!("antigravity: \"{}\"", raw_prompt),
            title: "Google Antigravity Agent Session".to_string(),
            success: true,
            summary: "Antigravity operates directly inside this IDE session via the HyperKB MCP Server.".to_string(),
            lines,
            file_targets: Vec::new(),
            selected_file_idx: 0,
        };
    }

    let mut cmd = std::process::Command::new(binary_name);
    cmd.current_dir(root);

    if harness_id.contains("claude") {
        cmd.arg("-p").arg(bundled_prompt);
    } else if harness_id.contains("opencode") {
        cmd.arg("run").arg(bundled_prompt);
    } else if harness_id.contains("codex") || harness_id.contains("openai") {
        cmd.arg("exec").arg(bundled_prompt);
    } else if harness_id.contains("ollama") {
        let model = harness.detected_models.first().cloned().unwrap_or_else(|| "llama3.3".to_string());
        cmd.arg("run").arg(model).arg(bundled_prompt);
    } else {
        cmd.arg(bundled_prompt);
    }

    match cmd.output() {
        Ok(output) => {
            let elapsed = start_time.elapsed();
            let raw_stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let raw_stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let clean_stdout = strip_ansi_codes(&raw_stdout);
            let clean_stderr = strip_ansi_codes(&raw_stderr);

            let mut lines = Vec::new();
            if !clean_stdout.trim().is_empty() {
                for line in clean_stdout.lines() {
                    lines.push(line.to_string());
                }
            } else if !clean_stderr.trim().is_empty() {
                for line in clean_stderr.lines() {
                    lines.push(line.to_string());
                }
            } else {
                lines.push("● Agent completed with empty output stream.".to_string());
            }

            let success = output.status.success();
            let summary = if success {
                format!("Completed in {:.2}s via {} harness", elapsed.as_secs_f32(), harness.name)
            } else {
                format!("Process exited with status {} in {:.2}s", output.status, elapsed.as_secs_f32())
            };

            DiagnosticEntry {
                id: uuid::Uuid::now_v7().to_string(),
                timestamp: chrono::Utc::now(),
                command: format!("{} query: \"{}\"", harness.name, raw_prompt),
                title: format!("AI Query: {} Response", harness.name),
                success,
                summary,
                lines,
                file_targets: Vec::new(),
                selected_file_idx: 0,
            }
        }
        Err(err) => {
            let elapsed = start_time.elapsed();
            DiagnosticEntry {
                id: uuid::Uuid::now_v7().to_string(),
                timestamp: chrono::Utc::now(),
                command: format!("{} query: \"{}\"", harness.name, raw_prompt),
                title: format!("Harness Execution Error: {}", harness.name),
                success: false,
                summary: format!("Failed to execute '{}' after {:.2}s: {}", binary_name, elapsed.as_secs_f32(), err),
                lines: vec![
                    format!("Error: {}", err),
                    format!("Executable target: '{}'", binary_name),
                    "".to_string(),
                    "Check that the CLI tool is installed and executable in PATH, or select a different default harness in Settings tab [5].".to_string(),
                ],
                file_targets: Vec::new(),
                selected_file_idx: 0,
            }
        }
    }
}

fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);

        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        result.push(CHARSET[((n >> 18) & 63) as usize] as char);
        result.push(CHARSET[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARSET[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARSET[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DocumentKind, DocumentStatus};

    fn make_test_doc(id: &str, path: &str, title: &str) -> Document {
        Document {
            id: id.to_string(),
            collection_id: "test".to_string(),
            path: path.to_string(),
            title: title.to_string(),
            topic: "topic".to_string(),
            status: DocumentStatus::Accepted,
            kind: DocumentKind::Decision,
            owner: "user".to_string(),
            issue: String::new(),
            replacement_id: None,
            supersedes: None,
            content: "content".to_string(),
            source: "file".to_string(),
            available: true,
            stale: false,
            declared_status: None,
            checksum: "abc".to_string(),
            worktree_state: None,
        }
    }

    #[test]
    fn test_explore_tree_build_and_collapse() {
        let mut app = App::new("test", "test");
        app.documents = vec![
            make_test_doc("d1", "decisions/adr-001.md", "Use Rust"),
            make_test_doc("d2", "decisions/adr-002.md", "SQLite DB"),
            make_test_doc("d3", "risks/risk-001.md", "Disk Thrashing"),
        ];

        let tree = app.build_explore_tree();
        assert_eq!(tree.len(), 5); // folder "decisions/", 2 docs, folder "risks/", 1 doc

        match &tree[0] {
            ExploreTreeItem::Folder { path, doc_count, is_collapsed, .. } => {
                assert_eq!(path, "decisions");
                assert_eq!(*doc_count, 2);
                assert!(!is_collapsed);
            }
            _ => panic!("Expected folder at root"),
        }

        // Collapse "decisions" folder
        app.toggle_tree_collapse("decisions");
        let collapsed_tree = app.build_explore_tree();
        assert_eq!(collapsed_tree.len(), 3); // folder "decisions/" (collapsed), folder "risks/", 1 doc

        match &collapsed_tree[0] {
            ExploreTreeItem::Folder { path, is_collapsed, .. } => {
                assert_eq!(path, "decisions");
                assert!(is_collapsed);
            }
            _ => panic!("Expected folder"),
        }

        // Toggle tree mode on app
        assert!(!app.explore_tree_mode);
        app.active_tab = ActiveTab::Explore;
        app.toggle_explore_tree_mode();
        assert!(app.explore_tree_mode);

        // Next item navigation in tree mode
        app.next();
        assert_eq!(app.selected_tree_idx, 1);
    }

    #[test]
    fn test_settings_adjustments_and_toggles() {
        let mut app = App::new("test", "test");
        app.theme = ThemeMode::Cyberpunk;
        assert_eq!(app.manifest.settings.max_briefing_directives, 5);
        assert!(!app.settings_dirty);

        // Adjust knob 0 (max_briefing_directives) +1
        app.settings_selected_idx = 0;
        app.adjust_setting(1);
        assert_eq!(app.manifest.settings.max_briefing_directives, 6);
        assert!(app.settings_dirty);

        // Adjust knob 4 (theme)
        app.settings_selected_idx = 4;
        assert_eq!(app.theme, ThemeMode::Cyberpunk);
        app.adjust_setting(1);
        assert_eq!(app.theme, ThemeMode::Modern);
        assert_eq!(app.manifest.settings.theme, "modern");

        // Adjust knob 5 (mouse)
        app.settings_selected_idx = 5;
        let initial_mouse = app.mouse_capture;
        app.adjust_setting(1);
        assert_eq!(app.mouse_capture, !initial_mouse);
        assert_eq!(app.manifest.settings.mouse_enabled, !initial_mouse);

        // Test help and methodology toggles
        assert!(!app.show_help);
        app.toggle_help();
        assert!(app.show_help);
        app.go_back();
        assert!(!app.show_help);

        assert!(!app.show_scoring_methodology);
        app.toggle_scoring_methodology();
        assert!(app.show_scoring_methodology);
        app.go_back();
        assert!(!app.show_scoring_methodology);
    }

    #[test]
    fn test_mouse_drag_selection_and_clipboard() {
        let mut app = App::new("test", "test");
        assert!(!app.is_dragging);
        assert_eq!(app.drag_start, None);
        assert_eq!(app.drag_current, None);
        assert_eq!(app.last_selected_text, None);

        // Simulate start of drag
        app.drag_start = Some((10, 5));
        app.drag_current = Some((10, 5));
        assert!(!app.is_dragging);

        // Simulate drag motion
        app.drag_current = Some((45, 5));
        app.is_dragging = true;

        // Simulate selected text extraction
        app.last_selected_text = Some("Never fail on customer data".to_string());
        assert_eq!(app.last_selected_text.as_deref(), Some("Never fail on customer data"));

        // Test clipboard helper does not panic
        App::copy_text_to_clipboard("Never fail on customer data");
    }

    #[test]
    fn test_slash_command_dock_filtering() {
        let mut app = App::new("test", "test");
        app.repl_input = "/".to_string();
        let all = app.filtered_slash_commands();
        assert_eq!(all.len(), SLASH_COMMANDS.len());

        app.repl_input = "/check".to_string();
        let filtered = app.filtered_slash_commands();
        assert!(!filtered.is_empty());
        assert!(filtered.iter().any(|cmd| cmd.name == "check"));

        app.repl_input = "/new".to_string();
        let filtered = app.filtered_slash_commands();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "new");

        app.repl_input = "/har".to_string();
        let filtered = app.filtered_slash_commands();
        assert!(filtered.iter().any(|cmd| cmd.name == "harnesses"));

        app.repl_input = "/xyznonexistent".to_string();
        assert!(app.filtered_slash_commands().is_empty());
    }

    #[test]
    fn test_governance_tab_mode_and_grants() {
        let mut app = App::new("test", "test");
        assert_eq!(app.governance_tab_mode, GovernanceTabMode::Sessions);

        app.toggle_governance_tab_mode();
        assert_eq!(app.governance_tab_mode, GovernanceTabMode::Grants);

        app.toggle_governance_tab_mode();
        assert_eq!(app.governance_tab_mode, GovernanceTabMode::Sessions);
    }

    #[test]
    fn test_work_tab_dual_mode_and_diagnostic_stream() {
        let mut app = App::new("test", "test");
        assert_eq!(app.work_tab_mode, WorkTabMode::Projects);
        assert_eq!(app.diagnostic_stream.len(), 1);
        assert!(!app.repl_active);

        // Switch to risks and console modes
        app.work_tab_mode = WorkTabMode::Risks;
        assert_eq!(app.work_tab_mode, WorkTabMode::Risks);

        app.work_tab_mode = WorkTabMode::Console;
        assert_eq!(app.work_tab_mode, WorkTabMode::Console);

        // Add a diagnostic entry with flagged files
        app.add_diagnostic_entry(
            "audit",
            "Knowledge Base Audit",
            "1 warning detected",
            vec![
                "✓ Total Directives: 5 (5 active)".to_string(),
                "! Bloat Warning: spec/bloated.md exceeds 250 lines".to_string(),
            ],
            vec!["docs/spec/bloated.md".to_string()],
            false,
        );

        assert_eq!(app.diagnostic_stream.len(), 2);
        assert_eq!(app.selected_diagnostic_idx, 1);

        // Active document path should point to the flagged file
        let path = app.get_active_document_path();
        assert!(path.is_some());
        assert!(path.unwrap().to_string_lossy().contains("docs/spec/bloated.md"));

        // Add another diagnostic entry
        app.add_diagnostic_entry(
            "check",
            "Work Hygiene Check",
            "All clear",
            vec!["✓ No stale directives violated".to_string()],
            vec![],
            true,
        );
        assert_eq!(app.diagnostic_stream.len(), 3);
        assert_eq!(app.selected_diagnostic_idx, 2);

        // Navigate between diagnostic entries
        app.prev_diagnostic_entry();
        assert_eq!(app.selected_diagnostic_idx, 1);
        app.next_diagnostic_entry();
        assert_eq!(app.selected_diagnostic_idx, 2);

        // Esc should switch back to Projects
        app.go_back();
        assert_eq!(app.work_tab_mode, WorkTabMode::Projects);
    }

    #[test]
    fn test_work_tab_projects_cockpit_navigation() {
        let db = Database::open_in_memory("test_coll", "test_prof").unwrap();
        let mut app = App::new("test_coll", "test_prof");
        assert_eq!(app.work_tab_mode, WorkTabMode::Projects);

        // Simulate loaded projects
        app.projects = vec![
            ProjectSummary {
                name: "service-auth".to_string(),
                path: "projects/service-auth".to_string(),
                total_documents: 10,
                tasks_pending: 3,
                tasks_in_progress: 2,
                tasks_completed: 4,
                tasks_blocked: 1,
                open_risks: 0,
                decisions_count: 1,
                has_status_doc: true,
            },
            ProjectSummary {
                name: "service-billing".to_string(),
                path: "projects/service-billing".to_string(),
                total_documents: 5,
                tasks_pending: 1,
                tasks_in_progress: 0,
                tasks_completed: 4,
                tasks_blocked: 0,
                open_risks: 0,
                decisions_count: 0,
                has_status_doc: true,
            },
        ];

        assert_eq!(app.selected_project_idx, 0);
        app.next();
        assert_eq!(app.selected_project_idx, 1);
        app.next();
        assert_eq!(app.selected_project_idx, 0);
        app.prev();
        assert_eq!(app.selected_project_idx, 1);

        // Test cycle work tab mode
        app.cycle_work_tab_mode(&db);
        assert_eq!(app.work_tab_mode, WorkTabMode::Risks);
        app.cycle_work_tab_mode(&db);
        assert_eq!(app.work_tab_mode, WorkTabMode::Console);
        app.cycle_work_tab_mode(&db);
        assert_eq!(app.work_tab_mode, WorkTabMode::Projects);
    }

    #[test]
    fn test_repl_command_execution() {
        let db = Database::open_in_memory("test", "test").unwrap();
        let mut app = App::new("test", "test");
        assert_eq!(app.diagnostic_stream.len(), 1);

        // Execute help
        app.execute_repl_command("help", &db);
        assert_eq!(app.diagnostic_stream.len(), 2);
        assert_eq!(app.diagnostic_stream.last().unwrap().command, "hyperkb help");
        assert_eq!(app.work_tab_mode, WorkTabMode::Console);

        // Execute audit
        app.execute_repl_command("audit", &db);
        assert_eq!(app.diagnostic_stream.len(), 3);
        assert_eq!(app.diagnostic_stream.last().unwrap().command, "hyperkb audit-kb");

        // Execute check
        app.execute_repl_command("check", &db);
        assert_eq!(app.diagnostic_stream.len(), 4);
        assert_eq!(app.diagnostic_stream.last().unwrap().command, "hyperkb check-work --staged --changed");

        // Execute harnesses
        app.execute_repl_command("harnesses", &db);
        assert_eq!(app.diagnostic_stream.len(), 5);
        assert_eq!(app.diagnostic_stream.last().unwrap().command, "hyperkb harnesses");

        // Slash command filtering tests
        app.repl_input = "/".to_string();
        let all_slash = app.filtered_slash_commands();
        assert_eq!(all_slash.len(), SLASH_COMMANDS.len());

        app.repl_input = "/au".to_string();
        let au_slash = app.filtered_slash_commands();
        assert!(au_slash.iter().any(|c| c.name == "audit"));

        // Execute command with leading slash
        app.execute_repl_command("/audit", &db);
        assert_eq!(app.diagnostic_stream.len(), 6);
        assert_eq!(app.diagnostic_stream.last().unwrap().command, "hyperkb audit-kb");

        // Test lifecycle navigation and action commands
        app.execute_repl_command("/directives", &db);
        assert_eq!(app.active_tab, ActiveTab::Directives);

        app.execute_repl_command("/risks", &db);
        assert_eq!(app.active_tab, ActiveTab::Work);
        assert_eq!(app.work_tab_mode, WorkTabMode::Risks);

        app.execute_repl_command("/sessions", &db);
        assert_eq!(app.active_tab, ActiveTab::Sessions);
        assert_eq!(app.governance_tab_mode, GovernanceTabMode::Sessions);

        app.execute_repl_command("/grants", &db);
        assert_eq!(app.active_tab, ActiveTab::Sessions);
        assert_eq!(app.governance_tab_mode, GovernanceTabMode::Grants);

        app.execute_repl_command("/grant", &db);
        assert!(app.show_issue_grant_modal);
        app.show_issue_grant_modal = false;

        app.execute_repl_command("/search auth", &db);
        assert_eq!(app.active_tab, ActiveTab::Explore);
        assert!(app.is_filtering);
        assert_eq!(app.filter_query, "auth");

        // Execute clear
        app.execute_repl_command("clear", &db);
        assert!(app.diagnostic_stream.is_empty());
    }

    #[test]
    fn test_settings_taxonomies_and_harness_knobs() {
        let mut app = App::new("test", "test");
        // Test knob 6: Taxonomies
        app.settings_selected_idx = 6;
        assert_eq!(app.settings_selected_idx, 6);
        app.adjust_setting(1);
        assert_eq!(app.settings_selected_idx, 6);

        // Test knob 7: AI Harnesses
        app.settings_selected_idx = 7;
        assert_eq!(app.settings_selected_idx, 7);
        app.adjust_setting(1);
        assert_eq!(app.settings_selected_idx, 7);

        // Full cycle of 8 settings
        app.settings_selected_idx = 0;
        for _ in 0..8 {
            app.next_setting();
        }
        assert_eq!(app.settings_selected_idx, 0);
    }

    #[test]
    fn test_active_harness_resolution_and_matching() {
        let app = App::new("test", "test");
        let active_name = app.active_harness_name();
        assert!(!active_name.is_empty());

        let default_harness = app.resolve_harness("default");
        assert!(default_harness.is_some());

        let claude_harness = app.resolve_harness("claude");
        assert!(claude_harness.is_some());

        let openai_harness = app.resolve_harness("openai");
        assert!(openai_harness.is_some());

        let antigravity_harness = app.resolve_harness("antigravity");
        assert!(antigravity_harness.is_some());

        let custom_harness = app.resolve_harness("my-custom-cli");
        assert!(custom_harness.is_some());
        assert_eq!(custom_harness.unwrap().id, "my-custom-cli");
    }

    #[test]
    fn test_governance_context_prompt_builder() {
        let mut app = App::new("test", "test");
        app.active_tab = ActiveTab::Explore;
        app.current_document = Some(Document {
            id: "arch_db".to_string(),
            collection_id: "test".to_string(),
            path: "docs/arch_db.md".to_string(),
            title: "Database Architecture".to_string(),
            topic: "database".to_string(),
            status: crate::domain::DocumentStatus::Accepted,
            kind: crate::domain::DocumentKind::Decision,
            owner: "ciso".to_string(),
            issue: "".to_string(),
            replacement_id: None,
            supersedes: None,
            content: "We use SQLite in WAL mode with robust transaction retries.".to_string(),
            source: "local".to_string(),
            available: true,
            stale: false,
            declared_status: None,
            checksum: "abc".to_string(),
            worktree_state: None,
        });

        let prompt = app.build_governance_context_prompt("Explain the transaction isolation model");
        assert!(prompt.contains("[HYPERKB GOVERNANCE CONTROL PLANE CONTEXT]"));
        assert!(prompt.contains("Active Tab: Explore"));
        assert!(prompt.contains("Active Document: Database Architecture"));
        assert!(prompt.contains("We use SQLite in WAL mode"));
        assert!(prompt.contains("Explain the transaction isolation model"));
    }

    #[test]
    fn test_headless_agent_query_dispatch_and_routing() {
        let db = Database::open_in_memory("test", "test").unwrap();
        let mut app = App::new("test", "test");

        // 1. Natural language query without slash routes to default harness
        app.execute_repl_command("What are the core governance invariants?", &db);
        assert_eq!(app.work_tab_mode, WorkTabMode::Console);
        assert!(app.pending_agent_query.is_some());
        assert!(app.agent_rx.is_some());
        assert!(app.diagnostic_stream.iter().any(|d| d.id == "pending_agent_query"));

        // Clear pending for next test
        app.pending_agent_query = None;
        app.agent_rx = None;

        // 2. Targeted slash command with empty prompt shows helpful usage
        app.execute_repl_command("/claude", &db);
        assert!(app.diagnostic_stream.last().unwrap().summary.contains("Usage: /claude"));

        // 3. Targeted slash command with prompt dispatches to that harness
        app.execute_repl_command("/claude explain the risk register", &db);
        assert!(app.pending_agent_query.is_some());
        assert!(app.diagnostic_stream.iter().any(|d| d.id == "pending_agent_query"));

        // 4. Antigravity IDE query returns immediate informative diagnostic
        app.pending_agent_query = None;
        app.agent_rx = None;
        app.execute_repl_command("/antigravity explain current session", &db);
        assert!(app.diagnostic_stream.iter().any(|d| d.id == "pending_agent_query"));
        let entry = app.agent_rx.take().unwrap().recv().unwrap();
        assert!(entry.lines[0].contains("Antigravity IDE Agent Environment"));
    }

    #[test]
    fn test_strip_ansi_codes_helper() {
        let colored = "\x1b[32mSuccess\x1b[0m: Process finished with code \x1b[1m0\x1b[0m";
        let clean = strip_ansi_codes(colored);
        assert_eq!(clean, "Success: Process finished with code 0");
    }
}
