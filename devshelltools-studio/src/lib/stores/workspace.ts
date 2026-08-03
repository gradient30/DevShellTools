import { writable } from "svelte/store";
import { api, type WorkspaceStatus } from "../api";

export const workspace = writable<WorkspaceStatus | null>(null);
export const loading = writable(false);
export const errorMsg = writable<string | null>(null);
export const successMsg = writable<string | null>(null);

export async function refresh() {
  loading.set(true);
  errorMsg.set(null);
  try {
    const s = await api.workspaceStatus();
    workspace.set(s);
    return s;
  } catch (e) {
    errorMsg.set(String(e));
    return null;
  } finally {
    loading.set(false);
  }
}

export async function init() {
  loading.set(true);
  errorMsg.set(null);
  successMsg.set(null);
  try {
    await api.initWorkspace();
    successMsg.set("工作区初始化成功，已创建首次 git 提交。");
    await refresh();
  } catch (e) {
    errorMsg.set(String(e));
  } finally {
    loading.set(false);
  }
}

export function clearMessages() {
  errorMsg.set(null);
  successMsg.set(null);
}