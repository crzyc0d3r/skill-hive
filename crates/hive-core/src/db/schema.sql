-- Skill Hive index. Rebuildable from the library directory (`hive doctor --reindex`).
-- Timestamps are integer milliseconds since the Unix epoch. JSON columns hold serde output.

CREATE TABLE IF NOT EXISTS meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS items (
  id                 TEXT PRIMARY KEY,
  kind               TEXT NOT NULL,            -- skill | agent | command
  name               TEXT NOT NULL,
  slug               TEXT NOT NULL,
  description        TEXT NOT NULL DEFAULT '',
  path               TEXT NOT NULL UNIQUE,
  content_hash       TEXT NOT NULL,
  size_bytes         INTEGER NOT NULL DEFAULT 0,
  token_estimate     INTEGER NOT NULL DEFAULT 0,
  description_tokens INTEGER NOT NULL DEFAULT 0,
  frontmatter_json   TEXT NOT NULL DEFAULT '{}',
  body               TEXT NOT NULL DEFAULT '',
  source_json        TEXT NOT NULL,
  validation         TEXT NOT NULL DEFAULT 'unknown',
  validation_json    TEXT NOT NULL DEFAULT '{"findings":[]}',
  effectiveness_json TEXT,
  favorite           INTEGER NOT NULL DEFAULT 0,
  notes              TEXT,
  created_at         INTEGER NOT NULL,
  updated_at         INTEGER NOT NULL,
  UNIQUE (kind, slug)
);
CREATE INDEX IF NOT EXISTS items_kind_name ON items(kind, name);
CREATE INDEX IF NOT EXISTS items_hash ON items(content_hash);

CREATE TABLE IF NOT EXISTS item_tags (
  item_id TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  tag     TEXT NOT NULL,
  PRIMARY KEY (item_id, tag)
);
CREATE INDEX IF NOT EXISTS item_tags_tag ON item_tags(tag);

-- Searchable projection of items; kept in step by the item repository inside the same transaction.
CREATE TABLE IF NOT EXISTS items_search (
  rowid       INTEGER PRIMARY KEY,
  item_id     TEXT NOT NULL UNIQUE REFERENCES items(id) ON DELETE CASCADE,
  name        TEXT NOT NULL,
  description TEXT NOT NULL,
  body        TEXT NOT NULL,
  tags        TEXT NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
  name, description, body, tags,
  content='items_search', content_rowid='rowid',
  tokenize='porter unicode61 remove_diacritics 2'
);
CREATE TRIGGER IF NOT EXISTS items_search_ai AFTER INSERT ON items_search BEGIN
  INSERT INTO items_fts(rowid, name, description, body, tags)
  VALUES (new.rowid, new.name, new.description, new.body, new.tags);
END;
CREATE TRIGGER IF NOT EXISTS items_search_ad AFTER DELETE ON items_search BEGIN
  INSERT INTO items_fts(items_fts, rowid, name, description, body, tags)
  VALUES ('delete', old.rowid, old.name, old.description, old.body, old.tags);
END;
CREATE TRIGGER IF NOT EXISTS items_search_au AFTER UPDATE ON items_search BEGIN
  INSERT INTO items_fts(items_fts, rowid, name, description, body, tags)
  VALUES ('delete', old.rowid, old.name, old.description, old.body, old.tags);
  INSERT INTO items_fts(rowid, name, description, body, tags)
  VALUES (new.rowid, new.name, new.description, new.body, new.tags);
END;

CREATE TABLE IF NOT EXISTS deployments (
  id            TEXT PRIMARY KEY,
  item_id       TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  target_id     TEXT NOT NULL,
  scope         TEXT NOT NULL,                 -- user | project
  project_path  TEXT,
  path          TEXT NOT NULL UNIQUE,
  mode          TEXT NOT NULL,                 -- link | copy
  deployed_hash TEXT NOT NULL,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS deployments_item ON deployments(item_id);
CREATE INDEX IF NOT EXISTS deployments_target ON deployments(target_id, scope, project_path);

CREATE TABLE IF NOT EXISTS projects (
  path     TEXT PRIMARY KEY,
  name     TEXT NOT NULL,
  added_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS custom_targets (
  id        TEXT PRIMARY KEY,
  spec_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS target_overrides (
  target_id  TEXT PRIMARY KEY,
  skills_dir TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS loadouts (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL UNIQUE,
  description TEXT NOT NULL DEFAULT '',
  color       TEXT,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS loadout_items (
  loadout_id TEXT NOT NULL REFERENCES loadouts(id) ON DELETE CASCADE,
  item_id    TEXT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  position   INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (loadout_id, item_id)
);

CREATE TABLE IF NOT EXISTS providers (
  id          TEXT PRIMARY KEY,
  config_json TEXT NOT NULL,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS eval_runs (
  id            TEXT PRIMARY KEY,
  item_id       TEXT REFERENCES items(id) ON DELETE SET NULL,
  skill_name    TEXT NOT NULL,
  skill_hash    TEXT NOT NULL,
  config_json   TEXT NOT NULL,
  status        TEXT NOT NULL,
  progress_json TEXT NOT NULL DEFAULT '{"total":0,"done":0,"failed":0}',
  workspace     TEXT NOT NULL,
  summary_json  TEXT,
  error         TEXT,
  created_at    INTEGER NOT NULL,
  started_at    INTEGER,
  finished_at   INTEGER
);
CREATE INDEX IF NOT EXISTS eval_runs_item ON eval_runs(item_id, created_at);

CREATE TABLE IF NOT EXISTS eval_case_results (
  id          TEXT PRIMARY KEY,
  run_id      TEXT NOT NULL REFERENCES eval_runs(id) ON DELETE CASCADE,
  case_id     INTEGER NOT NULL,
  arm         TEXT NOT NULL,
  repeat      INTEGER NOT NULL,
  result_json TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS eval_case_results_run ON eval_case_results(run_id);

CREATE TABLE IF NOT EXISTS settings (
  key        TEXT PRIMARY KEY,
  value_json TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cache (
  key        TEXT PRIMARY KEY,
  value_json TEXT NOT NULL,
  fetched_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS audit (
  id          INTEGER PRIMARY KEY AUTOINCREMENT,
  ts          INTEGER NOT NULL,
  action      TEXT NOT NULL,
  detail_json TEXT NOT NULL DEFAULT '{}'
);
