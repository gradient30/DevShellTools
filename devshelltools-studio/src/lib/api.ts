import { invoke } from "@tauri-apps/api/core";

export interface WorkspaceStatus {
  initialized: boolean;
  root: string;
  version: string;
  template_version: string;
  created_at: string;
  last_sync: string;
  missing_files: string[];
  public_files: string[];
}

export interface CommitInfo {
  oid: string;
  message: string;
  time: number;
}

export interface PsFunction {
  name: string;
  synopsis: string;
  first_example: string;
}

export interface CategoryMeta {
  name: string;
  title: string;
  description: string;
  aliases: string[];
}

export interface CategoryInfo {
  file_name: string;
  category: CategoryMeta;
  functions: PsFunction[];
}

export interface ConsistencyReport {
  ok: boolean;
  errors: string[];
  warnings: string[];
  actual_functions: string[];
  psd1_exports: string[];
  psm1_exports: string[];
  help_commands: string[];
}

export interface SafetyReport {
  ok: boolean;
  violations: string[];
}

export const api = {
  // 工作区
  workspaceStatus: () => invoke<WorkspaceStatus>("workspace_status"),
  initWorkspace: () => invoke<string>("init_workspace"),
  // 读取
  listPublicFiles: () => invoke<string[]>("list_public_files"),
  readWorkspaceFile: (rel: string) => invoke<string>("read_workspace_file", { rel }),
  listCategories: () => invoke<CategoryInfo[]>("list_categories"),
  readCategoryFile: (fileName: string) => invoke<string>("read_category_file", { fileName }),
  // CRUD
  writeWorkspaceFile: (rel: string, content: string, message: string) =>
    invoke<string>("write_workspace_file", { rel, content, message }),
  deleteWorkspaceFile: (rel: string, message: string) =>
    invoke<string>("delete_workspace_file", { rel, message }),
  createCategory: (fileName: string, content: string, message: string) =>
    invoke<string>("create_category", { fileName, content, message }),
  deleteCategory: (fileName: string, message: string) =>
    invoke<string>("delete_category", { fileName, message }),
  updateCategoryFile: (fileName: string, content: string, message: string) =>
    invoke<string>("update_category_file", { fileName, content, message }),
  syncPublic: (message: string) => invoke<string>("sync_public", { message }),
  // 校验
  consistencyCheck: () => invoke<ConsistencyReport>("consistency_check"),
  safetyCheck: (code: string) => invoke<SafetyReport>("safety_check", { code }),
  validatePsSyntax: (code: string) => invoke<void>("validate_ps_syntax", { code }),
  // Git
  gitLog: (n?: number) => invoke<CommitInfo[]>("git_log", { n }),
  gitResetHard: (oid: string) => invoke<void>("git_reset_hard", { oid }),
  gitSnapshot: (message: string) => invoke<string>("git_snapshot", { message })
};