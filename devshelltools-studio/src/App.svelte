<script lang="ts">
  import { onMount } from "svelte";
  import { workspace, loading, errorMsg, successMsg, refresh, init, clearMessages } from "./lib/stores/workspace";
  import { api, type CommitInfo } from "./lib/api";

  let commits = $state<CommitInfo[]>([]);
  let logsLoading = $state(false);

  onMount(async () => {
    await refresh();
    const s = $workspace;
    if (s?.initialized) await loadLog();
  });

  async function loadLog() {
    logsLoading = true;
    try {
      commits = await api.gitLog(10);
    } catch (e) {
      console.error(e);
    } finally {
      logsLoading = false;
    }
  }

  async function handleInit() {
    await init();
    await loadLog();
  }

  function fmtTime(secs: number): string {
    return new Date(secs * 1000).toLocaleString("zh-CN");
  }

  function shortOid(oid: string): string {
    return oid.slice(0, 8);
  }
</script>

<main class="min-h-screen p-6 max-w-5xl mx-auto">
  <header class="mb-6">
    <h1 class="text-2xl font-bold text-cyan-300">DevShellTools Studio</h1>
    <p class="text-sm text-slate-400 mt-1">便携命令管理 · 模板版本 1.0.5</p>
  </header>

  {#if $errorMsg}
    <div class="mb-4 p-3 rounded bg-red-900/50 border border-red-700 text-red-200 text-sm flex justify-between">
      <span>{$errorMsg}</span>
      <button class="ml-3 text-red-300 hover:text-red-100" onclick={clearMessages}>×</button>
    </div>
  {/if}
  {#if $successMsg}
    <div class="mb-4 p-3 rounded bg-green-900/50 border border-green-700 text-green-200 text-sm flex justify-between">
      <span>{$successMsg}</span>
      <button class="ml-3 text-green-300 hover:text-green-100" onclick={clearMessages}>×</button>
    </div>
  {/if}

  {#if $loading && !$workspace}
    <div class="text-slate-400">加载中…</div>
  {:else if !$workspace}
    <div class="text-slate-400">无法获取工作区状态。</div>
  {:else if !$workspace.initialized}
    <!-- 初始化向导 -->
    <section class="bg-slate-800/60 rounded-lg p-6 border border-slate-700">
      <h2 class="text-lg font-semibold text-amber-300 mb-2">首次使用</h2>
      <p class="text-sm text-slate-300 mb-1">未检测到工作区。</p>
      <p class="text-xs text-slate-400 mb-4">
        将在 <code class="text-cyan-300">{$workspace.root}</code> 初始化一份完整的 DevShellTools 1.0.5 模板，并自动建立 git 快照。
      </p>
      {#if $workspace.missing_files.length > 0}
        <p class="text-xs text-amber-400 mb-3">缺失文件：{$workspace.missing_files.join(", ")}</p>
      {/if}
      <button
        class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded text-sm font-medium disabled:opacity-50"
        onclick={handleInit}
        disabled={$loading}
      >
        {$loading ? "初始化中…" : "初始化工作区"}
      </button>
    </section>
  {:else}
    <!-- 已初始化：状态展示 -->
    <section class="bg-slate-800/60 rounded-lg p-5 border border-slate-700 mb-4">
      <div class="flex items-center justify-between mb-3">
        <h2 class="text-lg font-semibold text-green-300">工作区就绪</h2>
        <span class="text-xs text-slate-400">v{$workspace.version}</span>
      </div>
      <dl class="grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
        <dt class="text-slate-400">路径</dt>
        <dd class="text-slate-200 break-all"><code>{$workspace.root}</code></dd>
        <dt class="text-slate-400">模板版本</dt>
        <dd class="text-slate-200">{$workspace.template_version}</dd>
        <dt class="text-slate-400">创建时间</dt>
        <dd class="text-slate-200">{fmtTime(Date.parse($workspace.created_at) / 1000)}</dd>
        <dt class="text-slate-400">最近同步</dt>
        <dd class="text-slate-200">{fmtTime(Date.parse($workspace.last_sync) / 1000)}</dd>
      </dl>
    </section>

    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <!-- Public 文件 -->
      <section class="bg-slate-800/60 rounded-lg p-4 border border-slate-700">
        <h3 class="text-sm font-semibold text-cyan-300 mb-3">Public 命令文件 ({$workspace.public_files.length})</h3>
        <ul class="space-y-1 text-sm">
          {#each $workspace.public_files as f}
            <li class="text-slate-300 font-mono text-xs">📄 {f}</li>
          {/each}
        </ul>
      </section>

      <!-- Git 日志 -->
      <section class="bg-slate-800/60 rounded-lg p-4 border border-slate-700">
        <h3 class="text-sm font-semibold text-cyan-300 mb-3">Git 快照历史</h3>
        {#if logsLoading}
          <p class="text-xs text-slate-400">加载中…</p>
        {:else if commits.length === 0}
          <p class="text-xs text-slate-400">暂无提交</p>
        {:else}
          <ul class="space-y-2 text-xs">
            {#each commits as c}
              <li class="border-l-2 border-slate-600 pl-2">
                <div class="text-slate-400 font-mono">{shortOid(c.oid)} · {fmtTime(c.time)}</div>
                <div class="text-slate-200">{c.message}</div>
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    </div>
  {/if}

  <footer class="mt-8 text-xs text-slate-500 text-center">
    M1 骨架 · 工作区初始化 + git 快照
  </footer>
</main>