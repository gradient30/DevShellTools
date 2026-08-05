import { writable } from "svelte/store";
import { api, type InitProgress, type WorkspaceStatus } from "../api";

export const workspace = writable<WorkspaceStatus | null>(null);
export const loading = writable(false);
export const errorMsg = writable<string | null>(null);
export const successMsg = writable<string | null>(null);
export const initProgress = writable<InitProgress | null>(null);

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
  initProgress.set({ step: 0, label: "准备初始化…", percent: 0 });
  const unlisten = await api.onInitProgress((p) => initProgress.set(p));
  try {
    await api.initWorkspace();
    successMsg.set("工作区初始化成功。");
    await refresh();
  } catch (e) {
    errorMsg.set(String(e));
  } finally {
    unlisten();
    initProgress.set(null);
    loading.set(false);
  }
}

export function clearMessages() {
  errorMsg.set(null);
  successMsg.set(null);
}
