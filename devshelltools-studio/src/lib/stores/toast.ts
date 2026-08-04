import { writable } from "svelte/store";

export type ToastKind = "success" | "error" | "info";

export interface ToastItem {
  id: number;
  kind: ToastKind;
  message: string;
}

let nextId = 0;
export const toasts = writable<ToastItem[]>([]);

/** 顶部浮层提示，默认 4 秒后消失。 */
export function showToast(message: string, kind: ToastKind = "info", ms = 4000) {
  const id = ++nextId;
  toasts.update((items) => [...items, { id, kind, message }]);
  setTimeout(() => {
    toasts.update((items) => items.filter((t) => t.id !== id));
  }, ms);
}

export function dismissToast(id: number) {
  toasts.update((items) => items.filter((t) => t.id !== id));
}
