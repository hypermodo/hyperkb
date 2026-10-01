use crate::domain::{AgentSession, BrowseOptions, Directive, Document, RiskMatch};
use crate::storage::{Database, Queries};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Work = 0,
    Explore = 1,
    Directives = 2,
    Sessions = 3,
    Reader = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    List,
    Detail,
}

pub struct App {
    pub should_quit: bool,
    pub active_tab: ActiveTab,
    pub focused_pane: FocusedPane,
    pub collection_id: String,
    pub profile_id: String,

    // Explore / Document state
    pub documents: Vec<Document>,
    pub selected_doc_idx: usize,
    pub selected_category: String,

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

    // Search filter state
    pub is_filtering: bool,
    pub filter_query: String,
    pub status_message: Option<String>,
}

impl App {
    pub fn new(collection_id: &str, profile_id: &str) -> Self {
        Self {
            should_quit: false,
            active_tab: ActiveTab::Work,
            focused_pane: FocusedPane::List,
            collection_id: collection_id.to_string(),
            profile_id: profile_id.to_string(),
            documents: Vec::new(),
            selected_doc_idx: 0,
            selected_category: "all".into(),
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
        }
    }

    pub fn toggle_raw_view(&mut self) {
        self.show_raw = !self.show_raw;
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
                if self.focused_pane == FocusedPane::Detail {
                    self.session_preview_scroll += 2;
                } else if !self.sessions.is_empty() {
                    self.selected_session_idx = (self.selected_session_idx + 1) % self.sessions.len();
                    self.session_preview_scroll = 0;
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
                if self.focused_pane == FocusedPane::Detail {
                    self.session_preview_scroll += 15;
                } else if !self.sessions.is_empty() {
                    self.selected_session_idx = (self.selected_session_idx + 8).min(self.sessions.len() - 1);
                    self.session_preview_scroll = 0;
                }
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
                if self.focused_pane == FocusedPane::Detail {
                    self.session_preview_scroll = self.session_preview_scroll.saturating_sub(15);
                } else if !self.sessions.is_empty() {
                    self.selected_session_idx = self.selected_session_idx.saturating_sub(8);
                    self.session_preview_scroll = 0;
                }
            }
            ActiveTab::Reader => {
                self.reader_scroll_offset = self.reader_scroll_offset.saturating_sub(15);
            }
        }
    }

    pub fn open_selected(&mut self) {
        if self.active_tab == ActiveTab::Explore {
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
        }
    }

    pub fn go_back(&mut self) {
        if self.is_filtering {
            self.is_filtering = false;
            self.filter_query.clear();
        } else if self.active_tab == ActiveTab::Reader {
            self.active_tab = ActiveTab::Explore;
        }
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
}
