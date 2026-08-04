# AGENTS.md

本文件面向 AI 编码代理，描述 DevShellTools 仓库的结构、约定与工作流程。仓库主要文档与代码注释均使用中文。

## 项目概览

本仓库包含两个相关但独立的部分：

1. **DevShellTools（仓库根目录）**：一个 Windows PowerShell 模块（当前版本 1.0.4），提供开发与运维快捷命令（文件浏览、Git、网络诊断、代理管理、PowerShell 会话管理等）。目标运行时同时兼容 Windows PowerShell 5.1 与 PowerShell 7。命令行入口是 `dsh`（交互式分类菜单与帮助）。

2. **devshelltools-studio/（子目录）**：一个 Tauri 2 桌面应用（当前版本 1.0.5），是上述模块的可视化管理工具。它直接管理 `%USERPROFILE%\Documents\WindowsPowerShell\Modules\DevShellTools`（与 `install.ps1` 安装到 PS 5.1 的默认路径一致），提供分类/函数的编辑、AI 辅助生成、安全检查、一致性校验、PowerShell 语法校验、迁移/导出导入，并在每次写入后自动做 git 快照。元数据在 `.studio/` 子目录。

## 目录结构

```text
DevShellTools/
├─ DevShellTools.psd1        # 模块清单：版本、FunctionsToExport 导出列表
├─ DevShellTools.psm1        # 模块加载器：dot-source Private/ 与 Public/，再 Export-ModuleMember
├─ Private/Common.ps1        # 内部辅助函数（Write-Dst*、Assert-Dst*、Test-Dst* 等），不导出
├─ Public/                   # 按分类组织的公共命令脚本（Files/Git/Network/PowerShell/Proxy/Help）
├─ install.ps1               # 安装：复制到 5.1 与 7 的模块目录，并向 Profile 写入 Import-Module
├─ uninstall.ps1             # 卸载：删除模块目录并清理 Profile 行
└─ devshelltools-studio/
   ├─ package.json           # 前端：Svelte 5（runes）+ Vite 5 + Tailwind CSS 4 + @tauri-apps/api
   ├─ vite.config.ts         # 开发端口固定 1420（strictPort）
   ├─ svelte.config.js       # compilerOptions.runes = true（Svelte 5 runes 模式）
   ├─ src/                   # 前端源码（App.svelte + lib/api.ts + lib/stores + lib/components）
   ├─ templates/             # 模块模板：初始化工作区时整份复制（结构同根目录模块）
   └─ src-tauri/             # Rust 后端（Tauri 2，edition 2021，rust 1.77+）
      ├─ Cargo.toml          # tauri =2.8.5、tauri-plugin-fs、serde、thiserror、anyhow、chrono、log
      ├─ tauri.conf.json     # 窗口配置；bundle.active = false（默认不打包安装程序）
      ├─ src/                # lib.rs（命令注册）、commands、workspace、sync、ps_parser、
      │                      # consistency、safety、git、template、ai_*、migrate、export、
      │                      # logging、webview2、error、main
      └─ tests/              # common/（USERPROFILE 隔离锁）、m1~m4_acceptance.rs
```

## 构建与测试命令

PowerShell 模块本身无构建步骤，直接导入即可：

```powershell
Import-Module ./DevShellTools.psd1 -Force   # 从仓库根目录加载开发版
dsh                                          # 验证入口可用
```

Studio 应用（在 `devshelltools-studio/` 下，包管理器为 pnpm）：

```bash
pnpm install                # 安装前端依赖
pnpm dev                    # 仅启动 Vite 前端（端口 1420）
pnpm tauri dev              # 启动完整桌面应用（前端 + Rust 后端）
pnpm build                  # 构建前端到 dist/
pnpm tauri build            # 构建发布版应用

cd src-tauri
cargo test                  # 全部 Rust 测试：单元测试（src/ 内 #[cfg(test)]）+ tests/ 验收测试
cargo test --test m1_acceptance   # 单独跑 M1 验收
cargo test --test m2_acceptance   # 单独跑 M2 验收
cargo test --test m4_acceptance   # 单独跑 M4 验收
```

测试注意事项：验收测试通过 `tests/common/mod.rs` 的 `IsolatedProfile` 临时修改 `USERPROFILE` 把"工作区"隔离到临时目录（串行锁避免并行污染），并依赖系统 `git` 与 `powershell.exe`，因此测试只能在 Windows 上完整运行，且会启动真实子进程。模块目前没有 Pester 测试，验证靠手动导入模块并执行 `dsh` / 各快捷命令。

## 代码组织与约定

### PowerShell 模块（根目录）

- **加载机制**：`DevShellTools.psm1` 依次 dot-source `Private/*.ps1` 与 `Public/*.ps1`，然后按清单显式 `Export-ModuleMember`。**新增导出函数必须同时改三处**：`Public/` 中的函数定义、`DevShellTools.psm1` 的 `$exports` 数组、`DevShellTools.psd1` 的 `FunctionsToExport`；此外还要在 `Public/Help.ps1` 的帮助数据中登记（`dsh` 帮助系统依赖它）。`sync.rs` / `consistency.rs` 会校验这四处一致。
- **命名约定**：首字母小写的函数（如 `gs`、`lt`、`killport`）为公共导出命令；首字母大写的函数（如 `Assert-Git`、`Write-Dst*`、`Show-Dst*`）为内部辅助函数，不导出。该约定被 Studio 的 `sync.rs::is_exported` 硬编码依赖。
- **注释风格**：每个公共函数带 PowerShell 标准注释块 `<# .SYNOPSIS ... .EXAMPLE ... #>`，内容为中文；模块顶部 `Set-StrictMode -Version Latest`。
- **分类元数据**：`devshelltools-studio/templates/Public/*.ps1` 文件开头有 `@DST-Category ... @DST-Category-End` 注释块（字段：`Name`、`Title`、`Description`、`Aliases`），供 Studio 解析分类；无此块的文件（如 `Help.ps1`）视为公共部分。注意：根目录 `Public/` 是当前 1.0.4 模块源码，模板目录是 1.0.5 版本、带分类块，两者内容相近但不完全同步——修改模块时如需保持 Studio 可用，应同步更新 `templates/`。
- **显示输出**：面向用户的消息用 `Write-Dst*` 辅助函数（`[成功]`/`[警告]` 等中文前缀 + 颜色）；分类详情用固定宽度左对齐格式化，不用 `Format-Table -AutoSize`（避免中文宽度计算错位）。

### Studio 后端（src-tauri/src/）

- `commands.rs`：所有 `#[tauri::command]` 入口，注册于 `lib.rs`。
- `workspace.rs`：工作区固定为 `%USERPROFILE%\Documents\WindowsPowerShell\Modules\DevShellTools`（与 `install.ps1` PS5.1 目标一致；依赖 `USERPROFILE`，测试靠 `IsolatedProfile` 隔离）；元数据在 `.studio/workspace.json`。
- `migrate.rs`：从旧路径（`Documents\DevShellTools` 旧 Studio 沙箱、PS7 模块目录等，排除当前工作区）合并 `Public/*.ps1`。
- `ps_parser.rs`：不自己解析 PowerShell，而是把代码写入带 UTF-8 BOM 的临时文件，调用 `powershell.exe` 的 AST 解析并回传 JSON。**因此 Studio 及其测试仅能在 Windows 上运行**。
- `git.rs`：调用系统 `git` 命令行（非 libgit2），每次写入操作自动 `add -A` + commit（快照）。
- `sync.rs`：扫描分类后重新生成 `.psd1`/`.psm1` 的导出列表等"公共部分"，原子写入（全部成功才落盘）。
- `safety.rs`：静态安全扫描，规则与下方"安全边界"一致。
- 依赖管理：刻意保持最小依赖（标准库 + 系统 git/powershell），新增 crate 需对照 Cargo.toml 中的里程碑注释（M3 才引入 reqwest/keyring 等）。

### Studio 前端（src/）

- Svelte 5 runes 模式（`svelte.config.js` 已开启），状态用 `lib/stores/workspace.svelte.ts`（runes store）。
- 与后端的全部交互集中在 `lib/api.ts`（`@tauri-apps/api` 的 `invoke` 封装），新增 Tauri 命令时在此补充类型与封装。
- 样式用 Tailwind CSS 4（通过 `@tailwindcss/vite` 插件，`src/app.css`）。

## 安全边界（必须遵守）

模块的设计红线，新增快捷命令时不得违反（`safety.rs` 对 Studio 写入的代码强制执行同样的规则）：

- 不提供 `git push --force` / `git reset --hard` / `git clean -f`（真实删除）的快捷命令；`gclean` 只允许 `-nd`/`-ndx` dry-run。
- `lpr` 只修改当前进程的代理环境变量，禁止写 `User` 级环境变量（`SetEnvironmentVariable(..., "User")`）。
- `killport` 默认只显示进程，必须显式 `-Stop` 才终止；`Stop-Process` 须配合 `-Confirm` 或 `SupportsShouldProcess`。
- `super` 只能打开新的管理员窗口，不得悄悄提升当前窗口；`psb` 只改当前进程执行策略。
- 禁止 `Remove-Item -Recurse -Force` 式的危险删除（`uninstall.ps1` 等明确卸载场景除外）。
- `install.ps1` / `uninstall.ps1` 只动当前用户的模块目录与 Profile，不需要管理员权限；安装前会对旧目录做带时间戳的备份。

## 部署 / 发布

- 模块发布即运行 `install.ps1`（复制到 `Documents\WindowsPowerShell\Modules\DevShellTools` 与 `Documents\PowerShell\Modules\DevShellTools`，并写 Profile），无独立 CI/CD 配置。
- Studio 的 `tauri.conf.json` 中 `bundle.active = false`，默认只产出可执行文件、不生成安装包。
- 版本号分散在多处：`DevShellTools.psd1`（模块）、`README.md` 标题与变更记录、`install.ps1` 输出文案、`devshelltools-studio/package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`git.rs` 的 init commit 文案。升版本时需逐一核对。
