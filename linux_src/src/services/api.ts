import { invoke } from "@tauri-apps/api/core";
import type {
  Category,
  Configuration,
  ConfigurationDetail,
  ConfigurationView,
  DiffResult,
  DiscoveredConfig,
  FileContent,
  PathPreview,
  RestoreResult,
  Snapshot,
  SnapshotAllResult
} from "../types";

export const api = {
  scanConfigurations: () => invoke<DiscoveredConfig[]>("scan_configurations"),

  listConfigurations: () => invoke<ConfigurationView[]>("list_configurations"),

  getConfigurationDetail: (id: string) =>
    invoke<ConfigurationDetail | null>("get_configuration_detail", { id }),

  addDiscovered: (definitionId: string, confirmed: boolean) =>
    invoke<Configuration>("add_discovered", { definitionId, confirmed }),

  previewCustomPath: (path: string) => invoke<PathPreview>("preview_custom_path", { path }),

  addCustom: (name: string, path: string, category: Category, confirmed: boolean) =>
    invoke<Configuration>("add_custom", { name, path, category, confirmed }),

  removeConfiguration: (id: string) => invoke<void>("remove_configuration", { id }),

  snapshotConfiguration: (id: string, reason?: string) =>
    invoke<Snapshot | null>("snapshot_configuration", { id, reason: reason ?? null }),

  snapshotAll: () => invoke<SnapshotAllResult>("snapshot_all"),

  listHistory: (id: string) => invoke<Snapshot[]>("list_history", { id }),

  diffSnapshot: (id: string, commit: string) => invoke<DiffResult>("diff_snapshot", { id, commit }),

  diffWorking: (id: string) => invoke<DiffResult>("diff_working", { id }),

  restoreSnapshot: (id: string, commit: string) =>
    invoke<RestoreResult>("restore_snapshot", { id, commit }),

  listConfigurationFiles: (id: string) => invoke<string[]>("list_configuration_files", { id }),

  readConfigurationFile: (id: string, relativePath?: string) =>
    invoke<FileContent>("read_configuration_file", { id, relativePath: relativePath ?? null }),

  writeConfigurationFile: (id: string, content: string, relativePath?: string) =>
    invoke<void>("write_configuration_file", { id, relativePath: relativePath ?? null, content })
};

/** Tauri commands reject with the plain string produced by `CoreError`'s Display impl. */
export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return String(err);
}
