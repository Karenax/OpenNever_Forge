CREATE TABLE dependency_baselines (
  source_path TEXT PRIMARY KEY NOT NULL,
  report_json TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
