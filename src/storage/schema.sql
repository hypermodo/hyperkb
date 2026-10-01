-- Collections & Profiles
CREATE TABLE IF NOT EXISTS collections (
  id TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS profiles (
  id TEXT PRIMARY KEY
);

CREATE TABLE IF NOT EXISTS root_bindings (
  collection_id TEXT NOT NULL REFERENCES collections(id),
  profile_id TEXT NOT NULL REFERENCES profiles(id),
  last_root TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  PRIMARY KEY(collection_id, profile_id)
);

-- Documents Table & Effective View
CREATE TABLE IF NOT EXISTS documents (
  id INTEGER PRIMARY KEY,
  source_id TEXT NOT NULL UNIQUE,
  collection_id TEXT NOT NULL REFERENCES collections(id),
  path TEXT NOT NULL,
  topic TEXT NOT NULL DEFAULT 'General',
  title TEXT NOT NULL,
  content TEXT NOT NULL,
  search_text TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'unknown',
  kind TEXT NOT NULL DEFAULT 'document',
  owner TEXT NOT NULL DEFAULT '',
  issue TEXT NOT NULL DEFAULT '',
  risk_paths TEXT NOT NULL DEFAULT '[]',
  risk_versions TEXT NOT NULL DEFAULT '[]',
  risk_environments TEXT NOT NULL DEFAULT '[]',
  supersedes TEXT NOT NULL DEFAULT '',
  checksum TEXT NOT NULL,
  indexed_at TEXT NOT NULL,
  UNIQUE(collection_id, path)
);

CREATE INDEX IF NOT EXISTS documents_collection_path ON documents(collection_id, path);
CREATE INDEX IF NOT EXISTS documents_collection_topic ON documents(collection_id, topic);
CREATE INDEX IF NOT EXISTS documents_supersedes ON documents(collection_id, supersedes, status);

CREATE VIEW IF NOT EXISTS effective_documents AS
SELECT d.*,
  CASE WHEN replacements.count > 1 THEN 'conflict'
       WHEN replacements.count = 1 THEN 'superseded'
       ELSE d.status END AS effective_status,
  COALESCE(replacements.replacement_id, '') AS replacement_id
FROM documents d
LEFT JOIN (
  SELECT collection_id, supersedes, count(*) AS count, min(source_id) AS replacement_id
  FROM documents WHERE kind='decision' AND status='accepted' AND supersedes<>''
  GROUP BY collection_id, supersedes
) replacements ON replacements.collection_id=d.collection_id AND replacements.supersedes=d.source_id;

CREATE TABLE IF NOT EXISTS document_aliases (
  collection_id TEXT NOT NULL REFERENCES collections(id),
  old_path TEXT NOT NULL,
  source_id TEXT NOT NULL REFERENCES documents(source_id),
  reviewed_at TEXT NOT NULL,
  PRIMARY KEY(collection_id, old_path)
);

-- FTS5 Full-Text Search on Documents
CREATE VIRTUAL TABLE IF NOT EXISTS documents_fts USING fts5(
  path, title, search_text, content='documents', content_rowid='id', tokenize='porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS documents_ai AFTER INSERT ON documents BEGIN
  INSERT INTO documents_fts(rowid,path,title,search_text) VALUES(new.id,new.path,new.title,new.search_text);
END;

CREATE TRIGGER IF NOT EXISTS documents_ad AFTER DELETE ON documents BEGIN
  INSERT INTO documents_fts(documents_fts,rowid,path,title,search_text)
  VALUES('delete',old.id,old.path,old.title,old.search_text);
END;

CREATE TRIGGER IF NOT EXISTS documents_au AFTER UPDATE ON documents BEGIN
  INSERT INTO documents_fts(documents_fts,rowid,path,title,search_text)
  VALUES('delete',old.id,old.path,old.title,old.search_text);
  INSERT INTO documents_fts(rowid,path,title,search_text) VALUES(new.id,new.path,new.title,new.search_text);
END;

-- Private Memories Table & FTS5
CREATE TABLE IF NOT EXISTS private_memories (
  id INTEGER PRIMARY KEY,
  source_id TEXT NOT NULL UNIQUE,
  profile_id TEXT NOT NULL REFERENCES profiles(id),
  title TEXT NOT NULL,
  content TEXT NOT NULL,
  kind TEXT NOT NULL DEFAULT 'note',
  origin TEXT NOT NULL DEFAULT 'local',
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS private_memories_profile_updated ON private_memories(profile_id, updated_at);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
  title, content, content='private_memories', content_rowid='id', tokenize='porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON private_memories BEGIN
  INSERT INTO memories_fts(rowid,title,content) VALUES(new.id,new.title,new.content);
END;

CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON private_memories BEGIN
  INSERT INTO memories_fts(memories_fts,rowid,title,content)
  VALUES('delete',old.id,old.title,old.content);
END;

CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON private_memories BEGIN
  INSERT INTO memories_fts(memories_fts,rowid,title,content)
  VALUES('delete',old.id,old.title,old.content);
  INSERT INTO memories_fts(rowid,title,content) VALUES(new.id,new.title,new.content);
END;

-- Historical Legacy Documents Table & FTS5
CREATE TABLE IF NOT EXISTS legacy_documents (
  id INTEGER PRIMARY KEY,
  source_id TEXT NOT NULL UNIQUE,
  profile_id TEXT NOT NULL REFERENCES profiles(id),
  snapshot_sha256 TEXT NOT NULL,
  old_project_path TEXT NOT NULL,
  old_doc_id TEXT NOT NULL,
  path TEXT NOT NULL,
  title TEXT NOT NULL,
  content TEXT NOT NULL,
  checksum TEXT NOT NULL,
  UNIQUE(profile_id,snapshot_sha256,old_project_path,old_doc_id)
);

CREATE VIRTUAL TABLE IF NOT EXISTS legacy_documents_fts USING fts5(
  title,content,content='legacy_documents',content_rowid='id',tokenize='porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS legacy_documents_ai AFTER INSERT ON legacy_documents BEGIN
  INSERT INTO legacy_documents_fts(rowid,title,content) VALUES(new.id,new.title,new.content);
END;

CREATE TRIGGER IF NOT EXISTS legacy_documents_ad AFTER DELETE ON legacy_documents BEGIN
  INSERT INTO legacy_documents_fts(legacy_documents_fts,rowid,title,content)
  VALUES('delete',old.id,old.title,old.content);
END;

-- Operations, Risk Acknowledgements, & Ledger
CREATE TABLE IF NOT EXISTS scan_runs (
  id INTEGER PRIMARY KEY,
  collection_id TEXT NOT NULL REFERENCES collections(id),
  started_at TEXT NOT NULL,
  finished_at TEXT NOT NULL,
  scanned INTEGER NOT NULL,
  read_errors INTEGER NOT NULL,
  prune_refused INTEGER NOT NULL,
  coverage_complete INTEGER NOT NULL,
  git_head TEXT NOT NULL DEFAULT '',
  git_index TEXT NOT NULL DEFAULT ''
);

CREATE INDEX IF NOT EXISTS scan_runs_collection ON scan_runs(collection_id, id DESC);

CREATE TABLE IF NOT EXISTS risk_acknowledgements (
  profile_id TEXT NOT NULL REFERENCES profiles(id),
  risk_source_id TEXT NOT NULL,
  scope_key TEXT NOT NULL,
  risk_checksum TEXT NOT NULL,
  reason TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY(profile_id,risk_source_id,scope_key)
);

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  checksum TEXT NOT NULL,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS legacy_imports (
  snapshot_sha256 TEXT PRIMARY KEY,
  archive_path TEXT NOT NULL,
  selected_paths TEXT NOT NULL,
  imported_rows INTEGER NOT NULL,
  imported_docs INTEGER NOT NULL DEFAULT 0,
  imported_at TEXT NOT NULL
);

-- SaaS Sync Outbox & State (Zero schema migrations needed when enabling Cloud Sync)
CREATE TABLE IF NOT EXISTS sync_outbox (
  mutation_id INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  op TEXT NOT NULL,
  hlc_ts TEXT NOT NULL,
  payload_json TEXT,
  synced_server_seq INTEGER DEFAULT NULL
);

CREATE INDEX IF NOT EXISTS sync_outbox_unsynced ON sync_outbox(synced_server_seq) WHERE synced_server_seq IS NULL;

CREATE TABLE IF NOT EXISTS sync_state (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- Session Metrics & Adaptive Learning Ledger
CREATE TABLE IF NOT EXISTS agent_sessions (
  id TEXT PRIMARY KEY,
  collection_id TEXT NOT NULL REFERENCES collections(id),
  profile_id TEXT NOT NULL REFERENCES profiles(id),
  agent_id TEXT NOT NULL,
  grant_id TEXT,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  total_tool_calls INTEGER NOT NULL DEFAULT 0,
  total_edits INTEGER NOT NULL DEFAULT 0,
  total_diff_lines INTEGER NOT NULL DEFAULT 0,
  risks_cited INTEGER NOT NULL DEFAULT 0,
  risks_prevented INTEGER NOT NULL DEFAULT 0,
  review_loops INTEGER NOT NULL DEFAULT 0,
  first_pass_clean INTEGER NOT NULL DEFAULT 1,
  status TEXT NOT NULL DEFAULT 'active'
);

CREATE INDEX IF NOT EXISTS agent_sessions_collection ON agent_sessions(collection_id, started_at DESC);

CREATE TABLE IF NOT EXISTS session_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES agent_sessions(id),
  timestamp TEXT NOT NULL,
  event_kind TEXT NOT NULL,
  target_path TEXT NOT NULL DEFAULT '',
  query_or_tool TEXT NOT NULL DEFAULT '',
  detail_json TEXT NOT NULL DEFAULT '{}'
);

CREATE INDEX IF NOT EXISTS session_events_session ON session_events(session_id, id ASC);
CREATE INDEX IF NOT EXISTS session_events_kind ON session_events(event_kind, timestamp DESC);


