CREATE TABLE world_reports (
  project_id TEXT PRIMARY KEY REFERENCES resource_catalogs(project_id) ON DELETE CASCADE,
  schema_version INTEGER NOT NULL,
  summary_json TEXT NOT NULL,
  report_json TEXT NOT NULL,
  diagnostic_count INTEGER NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
