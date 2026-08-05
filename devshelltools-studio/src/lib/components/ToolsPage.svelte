<script lang="ts">
  import { api, type MigrationCheck, type MigrateResult, type Webview2Status } from "../api";

  let {
    migration = null,
    webview2 = null,
    logFiles = [],
    loaded = false,
    onRefresh
  }: {
    migration: MigrationCheck | null;
    webview2: Webview2Status | null;
    logFiles: string[];
    loaded: boolean;
    onRefresh: () => void | Promise<void>;
  } = $props();

  let currentLog = $state("");
  let currentLogName = $state("");
  let busy = $state(false);
  let msg = $state("");
  let errMsg = $state("");

  async function doMigrate() {
    if (!migration?.has_legacy) return;
    if (
      !confirm(
        "将把较新的 Public 命令并入当前工作区，重生成公共部分并同步到 PowerShell 模块目录；旧 Studio 沙箱（Documents\\DevShellTools）会归档为 DevShellTools.migrated-*。是否继续？"
      )
    ) {
      return;
    }
    busy = true;
    errMsg = "";
    msg = "";
    try {
      const result: MigrateResult = await api.migrateLegacy();
      const files = result.migrated_files.length
        ? `更新文件：${result.migrated_files.join(", ")}`
        : "无需覆盖文件（工作区已是最新）";
      const archived = result.archived_dirs.length
        ? `\n已归档：${result.archived_dirs.join("；")}`
        : "";
      msg = `${result.message}\n${files}${archived}`;
      await onRefresh();
    } catch (e) {
      errMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function doExport() {
    const dir = prompt("输入导出目标目录（完整路径）：");
    if (!dir) return;
    busy = true;
    errMsg = "";
    msg = "";
    try {
      const path = await api.exportWorkspace(dir);
      msg = `已导出到：${path}`;
    } catch (e) {
      errMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function doImport() {
    const dir = prompt("输入导入源目录（含 .ps1 脚本的目录）：");
    if (!dir) return;
    if (!confirm("导入会逐个校验脚本语法和安全，通过才写入。确认导入？")) return;
    busy = true;
    errMsg = "";
    msg = "";
    try {
      const result = await api.importWorkspace(dir);
      const parts = [`导入 ${result.imported.length} 个`];
      if (result.skipped.length > 0) parts.push(`跳过 ${result.skipped.length} 个`);
      if (result.errors.length > 0) parts.push(`${result.errors.length} 个错误`);
      msg = parts.join("，");
      if (result.errors.length > 0) {
        errMsg = result.errors.join("\n");
      }
      await onRefresh();
    } catch (e) {
      errMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function selectLog(name: string) {
    currentLogName = name;
    try {
      currentLog = await api.readLog(name);
    } catch (e) {
      currentLog = String(e);
    }
  }

  async function openWebview2Download() {
    const url = await api.webview2DownloadUrl();
    window.open(url, "_blank");
  }
</script>

<div class="p-5 max-w-3xl mx-auto space-y-6">
  <div class="flex items-center justify-between">
    <h2 class="text-lg font-semibold text-cyan-300">工具箱</h2>
    <button class="text-xs text-slate-400 hover:text-cyan-300" onclick={() => onRefresh()} disabled={busy}>刷新</button>
  </div>

  {#if !loaded}
    <div class="space-y-3">
      <div class="h-24 bg-slate-800/40 rounded animate-pulse"></div>
      <div class="h-24 bg-slate-800/40 rounded animate-pulse"></div>
    </div>
  {:else}
    {#if msg}
      <div class="p-2 text-xs bg-green-900/40 border border-green-700 text-green-200 rounded">{msg}</div>
    {/if}
    {#if errMsg}
      <div class="p-2 text-xs bg-red-900/40 border border-red-700 text-red-200 rounded">{errMsg}</div>
    {/if}

    <section class="bg-slate-800/60 rounded-lg p-4 border border-slate-700">
      <h3 class="text-sm font-semibold text-amber-300 mb-2">旧版迁移助手</h3>
      <p class="text-xs text-slate-500 mb-3 leading-relaxed">
        当前工作区为最新编辑副本。迁移会合并其它位置中较新的命令，同步模块目录，并归档旧沙箱；完成后不应再提示「旧版安装」。
      </p>
      {#if migration?.has_legacy}
        <p class="text-xs text-slate-300 mb-2">仍需处理：</p>
        <ul class="text-xs text-slate-400 mb-3 space-y-1.5">
          {#each migration.legacy_dirs as d}
            <li class="font-mono break-all leading-snug pl-2 border-l-2 border-amber-700/60">{d}</li>
          {/each}
        </ul>
        <button
          class="px-3 py-1.5 text-sm bg-amber-600 hover:bg-amber-500 rounded disabled:opacity-50"
          onclick={doMigrate}
          disabled={busy}>
          {busy ? "迁移中…" : "执行迁移并清理旧版"}
        </button>
      {:else}
        <p class="text-xs text-emerald-400/90">未检测到待迁移的旧版内容，当前即为最新。</p>
      {/if}
    </section>

    <section class="bg-slate-800/60 rounded-lg p-4 border border-slate-700">
      <h3 class="text-sm font-semibold text-cyan-300 mb-2">导出 / 导入</h3>
      <p class="text-xs text-slate-400 mb-3">导出所有 Public/*.ps1 脚本到目录；从目录导入时逐个校验语法和安全，通过才写入。</p>
      <div class="flex gap-2">
        <button class="px-3 py-1.5 text-sm bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50" onclick={doExport} disabled={busy}>导出工作区</button>
        <button class="px-3 py-1.5 text-sm bg-slate-700 hover:bg-slate-600 rounded disabled:opacity-50" onclick={doImport} disabled={busy}>导入工作区</button>
      </div>
    </section>

    <section class="bg-slate-800/60 rounded-lg p-4 border border-slate-700">
      <h3 class="text-sm font-semibold text-emerald-300 mb-2">WebView2 Runtime</h3>
      {#if webview2?.installed}
        <p class="text-xs text-green-300">已安装，版本 {webview2.version}</p>
      {:else}
        <p class="text-xs text-amber-300 mb-3">未检测到 WebView2 Runtime（Win10 需要安装）</p>
        <button class="px-3 py-1.5 text-sm bg-emerald-600 hover:bg-emerald-500 rounded" onclick={openWebview2Download}>下载 Evergreen Runtime</button>
      {/if}
    </section>

    <section class="bg-slate-800/60 rounded-lg p-4 border border-slate-700">
      <h3 class="text-sm font-semibold text-slate-300 mb-2">操作日志</h3>
      {#if logFiles.length === 0}
        <p class="text-xs text-slate-500">暂无日志</p>
      {:else}
        <div class="flex gap-4">
          <ul class="text-xs text-slate-400 w-40">
            {#each logFiles as f}
              <li>
                <button class="w-full text-left hover:text-cyan-300 {currentLogName === f ? 'text-cyan-300' : ''}" onclick={() => selectLog(f)}>{f}</button>
              </li>
            {/each}
          </ul>
          <pre class="flex-1 text-xs font-mono bg-slate-950 border border-slate-700 rounded p-2 overflow-auto max-h-64 text-slate-300">{currentLog || "选择左侧日志文件查看"}</pre>
        </div>
      {/if}
    </section>
  {/if}
</div>
