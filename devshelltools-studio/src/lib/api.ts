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

export const api = {
  workspaceStatus: () => invoke<WorkspaceStatus>("workspace_status"),
  initWorkspace: () => invoke<string>("init_workspace"),
  listPublicFiles: () => invoke<string[]>("list_public_files"),
  readWorkspaceFile: (rel: string) => invoke<string>("read_workspace_file", { rel }),
  writeWorkspaceFile: (rel: string, content: string, message: string) =>
    invoke<string>("write_workspace_file", { rel, content, message }),
  deleteWorkspaceFile: (rel: string, message: string) =>
    invoke<string>("delete_workspace_file", { rel, message }),
  gitLog: (n?: number) => invoke<CommitInfo[]>("git_log", { n }),
  gitResetHard: (oid: string) => invoke<void>("git_reset_hard", { oid }),
  gitSnapshot: (message: string) => invoke<string>("git_snapshot", { message })
};