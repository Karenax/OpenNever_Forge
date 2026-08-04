CREATE TABLE resource_catalogs (
  project_id TEXT PRIMARY KEY NOT NULL,
  source_digest TEXT NOT NULL,
  indexed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  resource_count INTEGER NOT NULL,
  version_count INTEGER NOT NULL,
  shadowed_count INTEGER NOT NULL
);

CREATE TABLE resource_versions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT NOT NULL REFERENCES resource_catalogs(project_id) ON DELETE CASCADE,
  resref TEXT NOT NULL,
  resource_type INTEGER NOT NULL,
  source_kind TEXT NOT NULL,
  source_name TEXT NOT NULL,
  source_path TEXT NOT NULL,
  priority INTEGER NOT NULL,
  resource_offset INTEGER NOT NULL,
  resource_size INTEGER NOT NULL,
  sha256 TEXT,
  is_selected INTEGER NOT NULL CHECK (is_selected IN (0, 1)),
  UNIQUE(project_id, resref, resource_type, source_path, resource_offset)
);

CREATE TABLE structured_summaries (
  project_id TEXT PRIMARY KEY REFERENCES resource_catalogs(project_id) ON DELETE CASCADE,
  summary_json TEXT NOT NULL
);

CREATE INDEX resource_versions_lookup_idx
  ON resource_versions(project_id, resref, resource_type, is_selected);
CREATE INDEX resource_versions_source_idx
  ON resource_versions(project_id, source_kind, source_name);
