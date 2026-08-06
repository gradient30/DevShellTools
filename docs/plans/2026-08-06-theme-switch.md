# Studio 明/暗/彩主题切换 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 DevShellTools Studio 实现明/暗/彩三主题切换，并用语义色保证对比度。

**Architecture:** `data-theme` 挂在 `document.documentElement`；`app.css` 定义三套 CSS 变量并通过 Tailwind `@theme` 暴露为 `bg-dst-*` / `text-dst-*`；组件全面改用语义类；顶栏三格切换器 + `localStorage`。

**Tech Stack:** Svelte 5 runes、Tailwind CSS 4、Vite、localStorage

**设计文档：** `docs/plans/2026-08-06-theme-switch-design.md`

---

### Task 1: 主题 store + CSS token

**Files:**
- Create: `devshelltools-studio/src/lib/stores/theme.svelte.ts`
- Modify: `devshelltools-studio/src/app.css`
- Modify: `devshelltools-studio/src/main.ts`

**Steps:**
1. 实现 `Theme = "light" | "dark" | "color"`、`initTheme()`、`setTheme()`、`getTheme()`；读写 `localStorage` 键 `dst-theme`；设置 `document.documentElement.dataset.theme` 与 `color-scheme`。
2. 在 `app.css` 定义三套变量与 `@theme` 映射；全局 `select, option, input, textarea` 绑定菜单/输入 token。
3. `main.ts` 在 mount 前调用 `initTheme()`。
4. Commit: `feat(studio): 新增主题 token 与 theme store`

### Task 2: 顶栏主题三格切换

**Files:**
- Create: `devshelltools-studio/src/lib/components/ThemeSwitch.svelte`
- Modify: `devshelltools-studio/src/App.svelte`（header）

**Steps:**
1. 三格按钮「明|暗|彩」，当前主题高亮（accent）。
2. 挂到 header 导航区右侧。
3. Commit: `feat(studio): 顶栏增加明暗彩主题切换`

### Task 3: 壳层与共享组件语义色

**Files:**
- Modify: `App.svelte`, `ToastHost.svelte`, `BusyOverlay.svelte`, `NewCategoryDialog.svelte`, `CategoryList.svelte`, `CategoryEditor.svelte`, `CommandTable.svelte`, `ToolsPage.svelte`, `AiSettings.svelte`, `ChatPanel.svelte`

**Steps:**
1. 将 `bg-slate-*` / `text-slate-*` / `border-slate-*` / 硬编码按钮色替换为 `dst-*` 语义类（或少量保留状态色但配对 fg）。
2. 所有 `<select>` 依赖全局 option 样式，勿再写浅字浅底。
3. 抽查三主题对比度。
4. Commit: `refactor(studio): 组件改用主题语义色`

### Task 4: 验收构建与打包

**Steps:**
1. `pnpm build` 通过。
2. 人工对照：明/暗/彩下顶栏、下拉、按钮、聊天输入可读。
3. 按用户约定：验收通过后 `git commit`（若有剩余）+ `pnpm exec tauri build --no-bundle`。

---

## 验收清单

- [ ] 三主题切换即时生效且刷新保持
- [ ] select/option、按钮、正文、placeholder 均无同色失明
- [ ] release exe 已更新
