<script lang="ts">
  import { api, type MigrationCheck, type Webview2Status } from "../api";

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
    if (!confirm("将旧版安装的 Public/*.ps1 合并到便携工作区？公共部分会自动重生成。")) return;
    busy = true;
    errMsg = "";
    msg = "";
    try {
      const files = await api.migrateLegacy();
      msg = `迁移完成：${files.length} 个文件`;
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
    const dir = prompt("输入导入源目录（完整路径，含 DevShellTools.psd1）：");
    if (!dir) return;
    if (!confirm("导入会覆盖当前工作区内容（.git 保留）。确认？")) return;
    busy = true;
    errMsg = "";
    msg = "";
    try {
      const files = await api.importWorkspace(dir);
      msg = `已导入：${files.length} 项`;
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
      {#if migration?.has_legacy}
        <p class="text-xs text-slate-300 mb-2">检测到旧版安装：</p>
        <ul class="text-xs text-slate-400 mb-2">
          {#each migration.legacy_dirs as d}<li class="font-mono">{d}</li>{/each}
        </ul>
        <button class="px-3 py-1.5 text-sm bg-amber-600 hover:bg-amber-500 rounded disabled:opacity-50" onclick={doMigrate} disabled={busy}>执行迁移</button>
      {:else}
        <p class="text-xs text-slate-500">未检测到旧版安装，无需迁移。</p>
      {/if}
    </section>

    <section class="bg-slate-800/60 rounded-lg p-4 border border-slate-700">
      <h3 class="text-sm font-semibold text-cyan-300 mb-2">导出 / 导入</h3>
      <p class="text-xs text-slate-400 mb-3">导出整个工作区到目录（不含 .git）；从目录导入覆盖当前工作区。</p>
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
