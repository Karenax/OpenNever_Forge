CREATE TABLE scripts (
  project_id TEXT NOT NULL REFERENCES resource_catalogs(project_id) ON DELETE CASCADE,
  resref TEXT NOT NULL,
  has_nss INTEGER NOT NULL CHECK (has_nss IN (0, 1)),
  has_ncs INTEGER NOT NULL CHECK (has_ncs IN (0, 1)),
  source_text TEXT,
  source_path TEXT,
  bytecode_path TEXT,
  line_count INTEGER NOT NULL,
  symbol_count INTEGER NOT NULL,
  diagnostic_count INTEGER NOT NULL,
  PRIMARY KEY(project_id, resref)
);

CREATE TABLE script_symbols (
  project_id TEXT NOT NULL,
  script_resref TEXT NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  line INTEGER NOT NULL,
  declaration TEXT NOT NULL,
  FOREIGN KEY(project_id, script_resref) REFERENCES scripts(project_id, resref) ON DELETE CASCADE
);

CREATE TABLE script_includes (
  project_id TEXT NOT NULL,
  script_resref TEXT NOT NULL,
  include_resref TEXT NOT NULL,
  line INTEGER NOT NULL,
  resolved INTEGER NOT NULL CHECK (resolved IN (0, 1)),
  FOREIGN KEY(project_id, script_resref) REFERENCES scripts(project_id, resref) ON DELETE CASCADE
);

CREATE TABLE script_references (
  project_id TEXT NOT NULL,
  script_resref TEXT NOT NULL,
  resource_resref TEXT NOT NULL,
  resource_type INTEGER NOT NULL,
  field_path TEXT NOT NULL,
  source_path TEXT NOT NULL,
  FOREIGN KEY(project_id, script_resref) REFERENCES scripts(project_id, resref) ON DELETE CASCADE
);

CREATE INDEX scripts_search_idx ON scripts(project_id, resref);
CREATE INDEX script_symbols_name_idx ON script_symbols(project_id, name);
CREATE INDEX script_references_target_idx ON script_references(project_id, script_resref);
