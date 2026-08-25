export type Category =
  | "shell"
  | "git"
  | "terminal"
  | "editor"
  | "ssh"
  | "package_managers"
  | "developer_tools"
  | "other";

export const CATEGORY_LABELS: Record<Category, string> = {
  shell: "Shell",
  git: "Git",
  terminal: "Terminal",
  editor: "Editor",
  ssh: "SSH",
  package_managers: "Package Managers",
  developer_tools: "Developer Tools",
  other: "Other"
};

export const CATEGORY_ORDER: Category[] = [
  "shell",
  "git",
  "terminal",
  "editor",
  "ssh",
  "package_managers",
  "developer_tools",
  "other"
];

export type Sensitivity = "normal" | "potential_secret" | "highly_sensitive";

export type ConfigKind = "file" | "directory";

export type Status = "synced" | "modified" | "not_tracked" | "missing" | "warning";

export const STATUS_LABELS: Record<Status, string> = {
  synced: "Synced",
  modified: "Modified",
  not_tracked: "Not managed",
  missing: "Missing",
  warning: "Warning"
};

export interface DiscoveredConfig {
  definition_id: string;
  application: string;
  category: Category;
  kind: ConfigKind;
  path: string;
  sensitivity: Sensitivity;
  is_private_key: boolean;
}

export interface Configuration {
  id: string;
  definition_id: string | null;
  name: string;
  path: string;
  category: Category;
  kind: ConfigKind;
  sensitivity: Sensitivity;
  added_at: string;
  last_snapshot_at: string | null;
  archived: boolean;
}

export interface ConfigurationView {
  configuration: Configuration;
  status: Status;
}

export interface ConfigurationDetail {
  configuration: Configuration;
  status: Status;
  size_bytes: number | null;
}

export interface Snapshot {
  id: string;
  configuration_id: string;
  created_at: string;
  git_commit: string;
  reason: string;
  favorite: boolean;
  archived: boolean;
}

export type FileChangeKind = "added" | "modified" | "deleted";
export type LineTag = "context" | "added" | "removed";

export interface DiffLine {
  tag: LineTag;
  content: string;
}

export interface FileDiff {
  path: string;
  change: FileChangeKind;
  binary: boolean;
  lines: DiffLine[];
}

export interface DiffResult {
  files: FileDiff[];
}

export interface SnapshotOutcome {
  configuration_id: string;
  name: string;
  snapshot: Snapshot | null;
}

export interface SnapshotAllResult {
  snapshotted: SnapshotOutcome[];
  unchanged_count: number;
}

export interface RestoreResult {
  backup_commit: string | null;
  restore_commit: string;
  verified: boolean;
}

export interface PathPreview {
  exists: boolean;
  is_directory: boolean;
  is_private_key: boolean;
  sensitivity: Sensitivity;
}

export interface FileContent {
  relative_path: string | null;
  content: string;
  is_binary: boolean;
}

export interface CatalogSuggestion {
  definition_id: string;
  application: string;
  category: Category;
  kind: ConfigKind;
  path: string;
  sensitivity: Sensitivity;
}

export interface DashboardSummary {
  configuration_count: number;
  file_count: number;
  total_size_bytes: number;
  modified_count: number;
  missing_count: number;
  snapshot_count: number;
}

/** Suggested content to append to an already-tracked file (see the editor's "Suggestions" panel). */
export interface SnippetSuggestion {
  label: string;
  description: string;
  snippet: string;
}
