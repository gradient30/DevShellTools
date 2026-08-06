# Studio 明/暗/彩主题切换 — 设计说明

**日期**：2026-08-06  
**状态**：已批准（用户确认：方案 1 + 彩=A）  
**产品**：DevShellTools Studio

## 目标

提供 **明 / 暗 / 彩** 三种主题网格切换；切换后背景、正文、按钮、下拉/option、placeholder 等前景/背景对比度始终可读，杜绝「同色看不见字」。

## 决策

| 项 | 选择 |
|----|------|
| 架构 | CSS 变量 + `data-theme` + Tailwind `@theme` 语义色 |
| 彩主题 | A：深色底 + 多色强调（青/琥珀/翠绿），终端/工具感 |
| 持久化 | `localStorage` 键 `dst-theme` |
| 默认 | `dark`（贴近现网） |
| 切换 UI | 顶栏右侧三格（明\|暗\|彩） |

## 对比度硬规则

1. 所有可读文字必须使用语义 token（`fg` / `fg-muted` / `accent-fg` / `danger-fg` 等），禁止组件内随意拼互不关联的 `slate-*` 灰阶。
2. 成对定义：`surface`↔`fg`、`btn`↔`btn-fg`、`menu`↔`menu-fg`、`input`↔`input-fg`、`placeholder`。
3. 原生 `select` / `option` 用全局规则绑定 `menu` token，并设置 `color-scheme`。
4. 状态色（danger/success/warning）各自带匹配前景色；三主题分别校准。
5. 验收：三主题下抽查顶栏、下拉、主按钮、次要按钮、聊天输入、Toast —— 肉眼可读。

## Token 清单（语义）

- 表面：`--dst-bg` / `--dst-surface` / `--dst-elevated` / `--dst-muted`
- 文字：`--dst-fg` / `--dst-fg-muted` / `--dst-fg-subtle`
- 边框：`--dst-border`
- 主操作：`--dst-accent` / `--dst-accent-fg` / `--dst-accent-hover`
- 输入：`--dst-input-bg` / `--dst-input-fg` / `--dst-placeholder`
- 菜单：`--dst-menu-bg` / `--dst-menu-fg` / `--dst-menu-hover`
- 状态：`--dst-danger*` / `--dst-success*` / `--dst-warning*`

## 非目标

- 不跟随系统 `prefers-color-scheme` 自动切换（可后续加）
- 不引入第三方主题库
- 不改后端 / Tauri 命令

## 验收标准

1. 顶栏可一切换三主题，刷新后保持。
2. 三主题下主界面、设置页下拉、AI 助手、工具箱文字与控件均清晰可读。
3. 现有功能行为不变；`pnpm build` 成功并可打 release 包。
