use crate::domain::{
    AgentSession, AuthorityGrant, BrowseOptions, Directive, Document, RepoManifest, RiskMatch,
};
use crate::storage::{Database, Queries};
use crate::ui::theme::ThemeMode;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceTabMode {
    Sessions,
    Grants,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionPaletteItem {
    pub id: &'static str,
    pub title: &'static str,
    pub shortcut: &'static str,
    pub description: &'static str,
}

pub static ACTION_PALETTE_ITEMS: &[ActionPaletteItem] = &[
    ActionPaletteItem {
        id: "check_work",
        title: "Run Git Check-Work (Audit Staged & Changed Files)",
        shortcut: "Space / c",
        description: "Audit working tree and index against cited risks and architectural directives",
    },
    ActionPaletteItem {
        id: "new_directive",
        title: "New Policy Directive (Wizard)",
        shortcut: "n",
        description: "Define a new repo invariant, behavior rule, or security guardrail",
    },
    ActionPaletteItem {
        id: "issue_grant",
        title: "Issue Agent Authority Grant",
        shortcut: "n",
        description: "Delegate scoped capability tokens to AI agents and sub-agents",
    },
    ActionPaletteItem {
        id: "audit_kb",
        title: "Run KB & Directives Linter Audit",
        shortcut: "a",
        description: "Verify document bloat, file hierarchy depth, and schema validity",
    },
    ActionPaletteItem {
        id: "open_editor",
        title: "Open Active File in External Editor",
        shortcut: "o",
        description: "Launch current markdown document in external IDE (code / $EDITOR)",
    },
    ActionPaletteItem {
        id: "backup",
        title: "Create Point-in-Time Backup Snapshot",
        shortcut: "B",
        description: "Create an atomic verified snapshot in .hyperkb/backups",
    },
    ActionPaletteItem {
        id: "compact",
        title: "Compact Database & WAL Journal",
        shortcut: "C",
        description: "Vacuum SQLite database and truncate WAL log to reclaim disk space",
    },
    ActionPaletteItem {
        id: "toggle_theme",
        title: "Cycle Visual Theme",
        shortcut: "T",
        description: "Switch between Cyberpunk, Modern, Nord, Tokyo Night, and Light themes",
    },
    ActionPaletteItem {
        id: "toggle_mouse",
        title: "Toggle Mouse Mode",
        shortcut: "m",
        description: "Switch between Click Navigation (ON) and Native Drag Selection (OFF)",
    },
    ActionPaletteItem {
        id: "view_help",
        title: "View System Documentation & Shortcuts",
        shortcut: "? / F1",
        description: "Open the interactive documentation manual",
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
}

impl App {
    pub fn new(collection_id: &str, profile_id: &str) -> Self {
        let root = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
        let manifest = RepoManifest::load_or_default(&root);
        let theme = ThemeMode::from_id(&manifest.settings.theme);
        let mouse_capture = manifest.settings.mouse_enabled;

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

    pub fn next_directive_category(&mut self, db: &Database) {
        let current_pos = Self::DIRECTIVE_CATEGORIES
            .iter()
            .position(|&c| c == self.directive_category)
            .unwrap_or(0);
        let next_pos = (current_pos + 1) % Self::DIRECTIVE_CATEGORIES.len();
        self.set_directive_category(Self::DIRECTIVE_CATEGORIES[next_pos], db);
    }

    pub fn set_directive_category(&mut self, category: &str, db: &Database) {
        self.directive_category = category.to_string();
        self.selected_directive_idx = 0;
        self.directive_preview_scroll = 0;
        self.refresh_data(db);
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
                if !self.active_risks.is_empty() {
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
                if !self.active_risks.is_empty() {
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
            ActiveTab::Work => self.next(),
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
            ActiveTab::Work => self.prev(),
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
        } else if self.active_tab == ActiveTab::Reader {
            self.active_tab = ActiveTab::Explore;
        }
    }

    pub fn next_setting(&mut self) {
        self.settings_selected_idx = (self.settings_selected_idx + 1) % 6;
    }

    pub fn prev_setting(&mut self) {
        if self.settings_selected_idx == 0 {
            self.settings_selected_idx = 5;
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
        let categories = ["architecture", "behavior", "deployment", "security"];
        let category = categories.get(self.new_directive_category_idx).unwrap_or(&"behavior");
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
            ActiveTab::Work => self.selected_risk().map(|r| self.root.join(&r.document.path)),
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
                    Ok("Git Check-Work: Clean. Working tree and index have no pending changes.".to_string())
                } else {
                    match Queries::check_work(db.conn(), &self.collection_id, &all_files, None, None) {
                        Ok(report) => {
                            if report.matches.is_empty() && report.hygiene_warnings.is_empty() {
                                Ok(format!("Git Check-Work: Clean across {} changed file(s). No risks cited.", all_files.len()))
                            } else {
                                Ok(format!(
                                    "Git Check-Work: {} file(s) checked. Warnings: {} hygiene, {} risks cited.",
                                    all_files.len(),
                                    report.hygiene_warnings.len(),
                                    report.matches.len()
                                ))
                            }
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
            "issue_grant" => {
                self.switch_tab(ActiveTab::Sessions);
                self.governance_tab_mode = GovernanceTabMode::Grants;
                self.show_issue_grant_modal = true;
                self.new_grant_field = 0;
                Ok("Issue Agent Authority Grant Wizard opened".to_string())
            }
            "audit_kb" => {
                let docs_dir = self.root.join(&self.manifest.docs_root);
                let report = crate::core::KbLinter::audit_directory(&docs_dir).unwrap_or_default();
                let dir_report = crate::core::DirectiveWorkflow::audit_directives(&self.root, db.conn(), &self.collection_id);
                let dir_count = dir_report.map(|r| r.active_directives).unwrap_or(0);
                Ok(format!(
                    "KB Audit Complete: {} docs checked, {} issues flagged. Active Directives: {}.",
                    report.total_documents,
                    report.schema_errors.len() + report.bloat_warnings.len(),
                    dir_count
                ))
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
            "toggle_theme" => {
                self.next_theme();
                Ok(format!("Active Theme: {}", self.theme.as_str()))
            }
            "toggle_mouse" => {
                let on = self.toggle_mouse();
                Ok(format!("Mouse Mode: {}", if on { "ON (Click Navigation & Drag Copy)" } else { "OFF (Native Selection)" }))
            }
            "view_help" => {
                self.show_help = true;
                Ok("Documentation & Shortcuts opened".to_string())
            }
            _ => Err(format!("Unknown action '{}'", action_id)),
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

        app.action_palette_query = "xyznonexistent".to_string();
        assert!(app.filtered_actions().is_empty());
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
}
