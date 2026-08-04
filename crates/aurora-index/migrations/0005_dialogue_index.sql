CREATE TABLE dialogues (
  project_id TEXT NOT NULL REFERENCES resource_catalogs(project_id) ON DELETE CASCADE,
  resref TEXT NOT NULL,
  source_path TEXT NOT NULL,
  node_count INTEGER NOT NULL,
  link_count INTEGER NOT NULL,
  cycle_count INTEGER NOT NULL,
  diagnostic_count INTEGER NOT NULL,
  PRIMARY KEY(project_id, resref)
);
CREATE TABLE dialogue_nodes (
  project_id TEXT NOT NULL, dialogue_resref TEXT NOT NULL, node_id TEXT NOT NULL,
  kind TEXT NOT NULL, node_index INTEGER NOT NULL, display_text TEXT, speaker TEXT,
  comment TEXT, action_script TEXT, PRIMARY KEY(project_id, dialogue_resref, node_id),
  FOREIGN KEY(project_id, dialogue_resref) REFERENCES dialogues(project_id, resref) ON DELETE CASCADE
);
CREATE TABLE dialogue_links (
  project_id TEXT NOT NULL, dialogue_resref TEXT NOT NULL, link_id TEXT NOT NULL,
  source_node TEXT, target_node TEXT NOT NULL, condition_script TEXT, action_script TEXT,
  is_child INTEGER NOT NULL CHECK (is_child IN (0, 1)), broken INTEGER NOT NULL CHECK (broken IN (0, 1)),
  PRIMARY KEY(project_id, dialogue_resref, link_id),
  FOREIGN KEY(project_id, dialogue_resref) REFERENCES dialogues(project_id, resref) ON DELETE CASCADE
);
CREATE TABLE dialogue_references (
  project_id TEXT NOT NULL, dialogue_resref TEXT NOT NULL, resource_resref TEXT NOT NULL,
  resource_type INTEGER NOT NULL, field_path TEXT NOT NULL, source_path TEXT NOT NULL,
  FOREIGN KEY(project_id, dialogue_resref) REFERENCES dialogues(project_id, resref) ON DELETE CASCADE
);
CREATE INDEX dialogue_nodes_text_idx ON dialogue_nodes(project_id, dialogue_resref, speaker);
CREATE INDEX dialogue_links_script_idx ON dialogue_links(project_id, condition_script, action_script);
CREATE INDEX dialogue_references_idx ON dialogue_references(project_id, dialogue_resref);
