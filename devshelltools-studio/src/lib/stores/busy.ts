import { writable } from "svelte/store";

/** 全局耗时操作遮罩：保证点击后立刻有可见反馈 */
export const appBusy = writable<{ active: boolean; message: string }>({
  active: false,
  message: ""
});

/**
 * 显示遮罩并等待一帧绘制，再执行异步任务（避免「点完无响应好几秒」）。
 */
export async function withBusy<T>(
  message: string,
  task: () => Promise<T>
): Promise<T> {
  appBusy.set({ active: true, message });
  // 让出主线程，确保遮罩先上屏
  await new Promise<void>((r) => requestAnimationFrame(() => r()));
  await new Promise<void>((r) => setTimeout(r, 0));
  try {
    return await task();
  } finally {
    appBusy.set({ active: false, message: "" });
  }
}
