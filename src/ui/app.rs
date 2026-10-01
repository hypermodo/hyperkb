use crate::domain::{
    AgentSession, AuthorityGrant, BrowseOptions, Directive, Document, RepoManifest, RiskMatch,
};
use crate::storage::{Database, Queries};
use crate::ui::theme::ThemeMode;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkTabMode {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPaletteItem {
    pub id: &'static str,
    pub title: &'static str,
    pub shortcut: &'static str,
    pub description: &'static str,
    pub cli_command: &'static str,
}

pub static ACTION_PALETTE_ITEMS: &[ActionPaletteItem] = &[
    ActionPaletteItem {
        id: "check_work",
        title: "Git Check-Work (Audit Staged & Changed Files)",
        shortcut: "Space",
        description: "Audit working tree and index against cited risks and architectural directives",
        cli_command: "hyperkb check-work --staged --changed",
    },
    ActionPaletteItem {
        id: "new_directive",
        title: "Draft New Policy Directive",
        shortcut: "n",
        description: "Define a new repo invariant, behavior rule, or security guardrail",
        cli_command: "hyperkb draft-directive",
    },
    ActionPaletteItem {
        id: "toggle_directive_status",
        title: "Toggle Policy Directive Status (Active ⇄ Retired)",
        shortcut: "r",
        description: "Retire obsolete directive or reactivate rule into pre-commit enforcement gate",
        cli_command: "Directives Lifecycle (Active ⇄ Retired)",
    },
    ActionPaletteItem {
        id: "issue_grant",
        title: "Issue Agent Authority Grant",
        shortcut: "n",
        description: "Delegate scoped capability tokens to autonomous AI agents and sub-agents",
        cli_command: "hyperkb issue-grant",
    },
    ActionPaletteItem {
        id: "revoke_grant",
        title: "Revoke Selected Agent Authority Grant",
        shortcut: "r",
        description: "Immediately revoke and invalidate an agent capability grant token",
        cli_command: "hyperkb revoke-grant",
    },
    ActionPaletteItem {
        id: "audit_kb",
        title: "Audit Knowledge Base & Directives",
        shortcut: "a",
        description: "Verify document bloat, file hierarchy depth, and schema validity",
        cli_command: "hyperkb audit-kb",
    },
    ActionPaletteItem {
        id: "reindex_kb",
        title: "Re-index Knowledge Base (Incremental FTS5)",
        shortcut: "I",
        description: "Scan docs directory, parse frontmatter, and update full-text SQLite search index",
        cli_command: "hyperkb index",
    },
    ActionPaletteItem {
        id: "bootstrap_risks",
        title: "Bootstrap Risks from Git Incident Archeology",
        shortcut: "G",
        description: "Analyze git commit history for regression hotspots and draft proactive risk cards",
        cli_command: "hyperkb bootstrap",
    },
    ActionPaletteItem {
        id: "open_editor",
        title: "Open Active Document in External Editor",
        shortcut: "o",
        description: "Launch current markdown document in external IDE ($EDITOR / code)",
        cli_command: "code <path> / $EDITOR <path>",
    },
    ActionPaletteItem {
        id: "backup",
        title: "Create Point-in-Time Backup Snapshot",
        shortcut: "B",
        description: "Create an atomic verified snapshot in .hyperkb/backups/",
        cli_command: "hyperkb backup",
    },
    ActionPaletteItem {
        id: "compact",
        title: "Compact Database & WAL Journal",
        shortcut: "C",
        description: "Vacuum SQLite database and truncate WAL log to optimize storage",
        cli_command: "hyperkb compact",
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

    // Action Palette modal state
    pub show_action_palette: bool,
    pub action_palette_query: String,
    pub action_palette_selected_idx: usize,

    // Work / Cockpit tab mode
    pub work_tab_mode: WorkTabMode,

    // Live Diagnostic Output Stream & REPL Cockpit
    pub diagnostic_stream: Vec<DiagnosticEntry>,
    pub selected_diagnostic_idx: usize,
    pub diagnostic_scroll: usize,
    pub repl_input: String,
    pub repl_history: Vec<String>,
    pub repl_history_idx: usize,
    pub repl_active: bool,

    // AI Harness & LLM Registry
    pub harnesses: Vec<crate::domain::HarnessDefinition>,
    pub selected_harness_idx: usize,
}

impl App {
    pub fn new(collection_id: &str, profile_id: &str) -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
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
            command: "hyperkb cockpit".to_string(),
            title: "HyperKB Developer Cockpit Initialized".to_string(),
            success: true,
            summary: format!(
                "Governance kernel ready. Collection: '{}'. Rule ceiling: {}.",
                collection_id,
                manifest.settings.max_briefing_directives
            ),
            lines: vec![
                format!("Repo Manifest: loaded from {}", RepoManifest::FILE_NAME),
                format!("Harness Discovery: {} AI tool(s) registered/detected locally", harnesses.len()),
                "Type 'audit', 'check', 'reindex', 'directives', 'grants', 'harnesses', or 'help' below.".to_string(),
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
            show_action_palette: false,
            action_palette_query: String::new(),
            action_palette_selected_idx: 0,
            work_tab_mode: WorkTabMode::Risks,
            diagnostic_stream: vec![init_entry],
            selected_diagnostic_idx: 0,
            diagnostic_scroll: 0,
            repl_input: String::new(),
            repl_history: Vec::new(),
            repl_history_idx: 0,
            repl_active: false,
            harnesses,
            selected_harness_idx,
        }
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
    }

    pub const CATEGORIES: &'static [&'static str] = &["all", "decisions", "risks", "specs", "plans"];

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
                if self.work_tab_mode == WorkTabMode::Console {
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
                } else if !self.active_risks.is_empty() {
                    self.selected_risk_idx = (self.selected_risk_idx + 1) % self.active_risks.len();
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
                self.next_setting();
            }
            ActiveTab::Reader => {
                self.reader_scroll_offset += 2;
            }
        }
    }

    pub fn prev(&mut self) {
        match self.active_tab {
            ActiveTab::Work => {
                if self.work_tab_mode == WorkTabMode::Console {
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
                } else if !self.active_risks.is_empty() {
                    if self.selected_risk_idx == 0 {
                        self.selected_risk_idx = self.active_risks.len() - 1;
                    } else {
                        self.selected_risk_idx -= 1;
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
                self.prev_setting();
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
        } else if self.active_tab == ActiveTab::Work && self.work_tab_mode == WorkTabMode::Console {
            self.work_tab_mode = WorkTabMode::Risks;
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

    pub fn filtered_actions(&self) -> Vec<&'static ActionPaletteItem> {
        let q = self.action_palette_query.to_lowercase();
        ACTION_PALETTE_ITEMS
            .iter()
            .filter(|item| {
                if q.is_empty() {
                    true
                } else {
                    item.title.to_lowercase().contains(&q)
                        || item.description.to_lowercase().contains(&q)
                        || item.shortcut.to_lowercase().contains(&q)
                        || item.cli_command.to_lowercase().contains(&q)
                        || item.id.to_lowercase().contains(&q)
                }
            })
            .collect()
    }

    pub fn execute_action_palette_item(&mut self, action_id: &str, db: &Database) -> Result<String, String> {
        self.show_action_palette = false;
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
                let docs_dir = self.root.join(&self.manifest.docs_root);
                let report = crate::core::KbLinter::audit_directory_with_settings(
                    &docs_dir,
                    self.manifest.settings.audit_max_lines,
                    self.manifest.settings.audit_max_depth,
                    self.manifest.settings.stale_days_threshold,
                ).unwrap_or_default();
                let dir_report = crate::core::DirectiveWorkflow::audit_directives(&self.root, db.conn(), &self.collection_id);
                let dir_count = dir_report.as_ref().map(|r| r.active_directives).unwrap_or(0);
                let total_dirs = dir_report.as_ref().map(|r| r.total_directives).unwrap_or(0);
                let dormant_dirs = dir_report.as_ref().map(|r| r.dormant_directives.len()).unwrap_or(0);

                let mut file_targets = Vec::new();
                let mut lines = Vec::new();
                let total_issues = report.schema_errors.len() + report.bloat_warnings.len() + report.depth_warnings.len() + report.stale_warnings.len();

                lines.push(format!("Knowledge Base Root: '{}' ({} total documents)", self.manifest.docs_root, report.total_documents));
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
                let docs_dir = self.root.join(&self.manifest.docs_root);
                match crate::core::Scanner::index_directory(db.conn(), &docs_dir, &self.collection_id) {
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
                                format!("Documents scanned in '{}': {}", self.manifest.docs_root, rep.scanned),
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

    pub fn execute_repl_command(&mut self, cmd: &str, db: &Database) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }
        self.repl_history.push(trimmed.to_string());
        self.repl_history_idx = self.repl_history.len();
        self.repl_input.clear();

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let op = parts[0].to_lowercase();
        match op.as_str() {
            "audit" | "audit-kb" => {
                let _ = self.execute_action_palette_item("audit_kb", db);
            }
            "check" | "check-work" => {
                let _ = self.execute_action_palette_item("check_work", db);
            }
            "reindex" | "index" => {
                let _ = self.execute_action_palette_item("reindex_kb", db);
            }
            "bootstrap" => {
                let _ = self.execute_action_palette_item("bootstrap_risks", db);
            }
            "backup" => {
                let _ = self.execute_action_palette_item("backup", db);
            }
            "compact" => {
                let _ = self.execute_action_palette_item("compact", db);
            }
            "directives" => {
                self.switch_tab(ActiveTab::Directives);
            }
            "grants" => {
                self.switch_tab(ActiveTab::Sessions);
                self.governance_tab_mode = GovernanceTabMode::Grants;
            }
            "sessions" => {
                self.switch_tab(ActiveTab::Sessions);
                self.governance_tab_mode = GovernanceTabMode::Sessions;
            }
            "settings" => {
                self.switch_tab(ActiveTab::Settings);
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
            }
            "clear" => {
                self.diagnostic_stream.clear();
                self.selected_diagnostic_idx = 0;
                self.status_message = Some("Diagnostic stream cleared".to_string());
            }
            "help" => {
                let lines = vec![
                    "Supported Cockpit Commands:".to_string(),
                    "  • audit      - Run comprehensive KB anti-bloat, schema & directive audit".to_string(),
                    "  • check      - Audit staged/changed files against risks and directives".to_string(),
                    "  • reindex    - Re-index documents into SQLite full-text search index".to_string(),
                    "  • bootstrap  - Mine git log history to bootstrap candidate risks".to_string(),
                    "  • harnesses  - Inspect discovered AI harnesses and CISO governance status".to_string(),
                    "  • directives - Navigate to Directives & Policy Rules tab".to_string(),
                    "  • grants     - Navigate to Agent Authority Grants tab".to_string(),
                    "  • backup     - Create atomic verified database backup snapshot".to_string(),
                    "  • compact    - Run SQLite VACUUM and truncate WAL journal".to_string(),
                    "  • clear      - Clear diagnostic output stream".to_string(),
                    "  • help       - Show this command reference".to_string(),
                ];
                let entry = DiagnosticEntry {
                    id: uuid::Uuid::now_v7().to_string(),
                    timestamp: chrono::Utc::now(),
                    command: "hyperkb help".to_string(),
                    title: "Cockpit Command Help & Reference".to_string(),
                    success: true,
                    summary: "Reference guide for interactive cockpit REPL".to_string(),
                    lines,
                    file_targets: Vec::new(),
                    selected_file_idx: 0,
                };
                self.diagnostic_stream.push(entry);
                self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                self.work_tab_mode = WorkTabMode::Console;
                self.switch_tab(ActiveTab::Work);
            }
            unknown => {
                let entry = DiagnosticEntry {
                    id: uuid::Uuid::now_v7().to_string(),
                    timestamp: chrono::Utc::now(),
                    command: trimmed.to_string(),
                    title: format!("Unknown Command: '{}'", unknown),
                    success: false,
                    summary: format!("Command '{}' not recognized. Type 'help' for available commands.", unknown),
                    lines: vec![
                        "Type 'help' to see available cockpit commands, or press [Space] for Action Palette.".to_string(),
                    ],
                    file_targets: Vec::new(),
                    selected_file_idx: 0,
                };
                self.diagnostic_stream.push(entry);
                self.selected_diagnostic_idx = self.diagnostic_stream.len().saturating_sub(1);
                self.work_tab_mode = WorkTabMode::Console;
                self.status_message = Some(format!("Unknown command: '{}'", unknown));
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
    fn test_action_palette_filtering() {
        let mut app = App::new("test", "test");
        assert_eq!(app.filtered_actions().len(), ACTION_PALETTE_ITEMS.len());

        app.action_palette_query = "check".to_string();
        let filtered = app.filtered_actions();
        assert!(!filtered.is_empty());
        assert!(filtered.iter().any(|item| item.id == "check_work"));

        app.action_palette_query = "grant".to_string();
        let filtered = app.filtered_actions();
        assert!(filtered.iter().any(|item| item.id == "issue_grant"));
        assert!(filtered.iter().any(|item| item.id == "revoke_grant"));

        // Match by cli_command
        app.action_palette_query = "draft-directive".to_string();
        let filtered = app.filtered_actions();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "new_directive");

        // Match lifecycle actions: index and archeology
        app.action_palette_query = "index".to_string();
        let filtered = app.filtered_actions();
        assert!(filtered.iter().any(|item| item.id == "reindex_kb"));

        app.action_palette_query = "archeology".to_string();
        let filtered = app.filtered_actions();
        assert!(filtered.iter().any(|item| item.id == "bootstrap_risks"));

        // Match by shortcut
        app.action_palette_query = "Space".to_string();
        let filtered = app.filtered_actions();
        assert!(filtered.iter().any(|item| item.id == "check_work"));

        // Settings like theme and mouse must NOT be in action palette
        app.action_palette_query = "theme".to_string();
        assert!(app.filtered_actions().is_empty());
        app.action_palette_query = "toggle_mouse".to_string();
        assert!(app.filtered_actions().is_empty());

        app.action_palette_query = "xyznonexistent".to_string();
        assert!(app.filtered_actions().is_empty());

        // Test modal bounds safety
        let small_area = ratatui::layout::Rect::new(0, 0, 50, 15);
        let modal = crate::ui::views::ActionPaletteModal::modal_area(small_area);
        assert!(modal.width <= small_area.width);
        assert!(modal.height <= small_area.height);

        let large_area = ratatui::layout::Rect::new(0, 0, 160, 50);
        let modal = crate::ui::views::ActionPaletteModal::modal_area(large_area);
        assert!(modal.width <= 105);
        assert!(modal.height <= 26);
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
        assert_eq!(app.work_tab_mode, WorkTabMode::Risks);
        assert_eq!(app.diagnostic_stream.len(), 1);
        assert!(!app.repl_active);

        // Switch to console mode
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

        // Esc should switch back to Risks
        app.go_back();
        assert_eq!(app.work_tab_mode, WorkTabMode::Risks);
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
}
