import { getWorkspaceStatus, gitLog, type WorkspaceStatus, type CommitInfo } from "../api";

type Toast = { msg: string; kind: "info" | "error" | "ok"; id: number };

let workspaceState: WorkspaceStatus | null = $state(null);
let commitsState: CommitInfo[] = $state([]);
let loadingState = $state(false);
let errorState: string | null = $state(null);
let toasts: Toast[] = $state([]);

let toastId = 0;

export function getWorkspace() {
  return workspaceState;
}
export function getCommits() {
  return commitsState;
}
export function getLoading() {
  return loadingState;
}
export function getError() {
  return errorState;
}
export function getToasts() {
  return toasts;
}

export function showToast(msg: string, kind: Toast["kind"] = "ok") {
  const id = ++toastId;
  toasts = [...toasts, { msg, kind, id }];
  setTimeout(() => {
    toasts = toasts.filter((t) => t.id !== id);
  }, 3000);
}

export async function refresh() {
  loadingState = true;
  errorState = null;
  try {
    const s = await getWorkspaceStatus();
    workspaceState = s;
    if (s.initialized) {
      commitsState = await gitLog(10);
    } else {
      commitsState = [];
    }
  } catch (e: any) {
    errorState = String(e);
  } finally {
    loadingState = false;
  }
}

export function clearError() {
  errorState = null;
}