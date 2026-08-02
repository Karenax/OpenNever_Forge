CREATE TABLE projects (
  id TEXT PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  project_file_path TEXT,
  module_path TEXT NOT NULL,
  module_sha256 TEXT NOT NULL,
  read_only INTEGER NOT NULL CHECK (read_only = 1),
  created_at TEXT NOT NULL,
  last_opened_at TEXT NOT NULL
);

CREATE TABLE source_containers (
  id TEXT PRIMARY KEY NOT NULL,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  source_path TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  scan_state TEXT NOT NULL
);

CREATE TABLE diagnostics (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  correlation_id TEXT NOT NULL,
  code TEXT NOT NULL,
  severity TEXT NOT NULL,
  user_message TEXT NOT NULL,
  technical_message TEXT NOT NULL,
  source TEXT,
  resource TEXT,
  import_stage TEXT,
  suggestion TEXT
);

CREATE TABLE import_jobs (
  id TEXT PRIMARY KEY NOT NULL,
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  state TEXT NOT NULL,
  completed_units INTEGER NOT NULL DEFAULT 0,
  total_units INTEGER NOT NULL DEFAULT 0,
  error_code TEXT
);

CREATE INDEX diagnostics_project_id_idx ON diagnostics(project_id);
CREATE INDEX source_containers_project_id_idx ON source_containers(project_id);
