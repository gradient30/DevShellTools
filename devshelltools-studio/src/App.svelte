<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspace,
    loading,
    errorMsg,
    successMsg,
    refresh,
    init,
    clearMessages
  } from "./lib/stores/workspace";
  import { api, type CategoryInfo, type CommitInfo, type ConsistencyReport } from "./lib/api";
  import CategoryList from "./lib/components/CategoryList.svelte";
  import CategoryEditor from "./lib/components/CategoryEditor.svelte";
  import NewCategoryDialog from "./lib/components/NewCategoryDialog.svelte";
  import ChatPanel from "./lib/components/ChatPanel.svelte";
  import AiSettings from "./lib/components/AiSettings.svelte";

  type Tab = "manage" | "chat" | "settings";
  let tab = $state<Tab>("manage");

  let categories = $state<CategoryInfo[]>([]);
  let selectedFileName = $state<string | null>(null);
  let selectedCategory = $derived(categories.find((c) => c.file_name === selectedFileName) ?? null);
  let fileContent = $state("");
  let commits = $state<CommitInfo[]>([]);
  let consistency = $state<ConsistencyReport | null>(null);
  let showNewDialog = $state(false);
  let logsLoading = $state(false);
  let aiReady = $state(false);

  onMount(async () => {
    await refresh();
    const s = $workspace;
    if (s?.initialized) await loadAll();
  });

  async function loadAll() {
    await Promise.all([loadCategories(), loadLog(), loadConsistency(), loadAiReady()]);
  }

  async function loadCategories() {
    try {
      categories = await api.listCategories();
      if (!selectedFileName && categories.length > 0) {
        selectedFileName = categories[0].file_name;
        await loadFileContent();
      }
    } catch (e) {
      console.error(e);
    }
  }

  async function loadFileContent() {
    if (!selectedFileName) return;
    try {
      fileContent = await api.readCategoryFile(selectedFileName);
    } catch (e) {
      console.error(e);
    }
  }

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

  async function loadConsistency() {
    try {
      consistency = await api.consistencyCheck();
    } catch (e) {
      console.error(e);
    }
  }

  async function loadAiReady() {
    try {
      aiReady = await api.aiReady();
    } catch {
      aiReady = false;
    }
  }

  async function handleInit() {
    await init();
    await loadAll();
  }

  async function onSelect(fileName: string) {
    selectedFileName = fileName;
    await loadFileContent();
  }

  async function handleSave(content: string, message: string) {
    if (!selectedFileName) return;
    try {
      await api.updateCategoryFile(selectedFileName, content, message);
      successMsg.set("已保存并重生成公共部分");
      await loadAll();
      await loadFileContent();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function handleDelete(fileName: string) {
    if (!confirm(`确认删除分类文件 ${fileName}？此操作会自动重生成公共部分。`)) return;
    try {
      await api.deleteCategory(fileName, `删除分类 ${fileName}`);
      successMsg.set(`已删除 ${fileName}`);
      if (selectedFileName === fileName) selectedFileName = null;
      await loadAll();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function handleCreate(fileName: string, content: string, message: string) {
    try {
      await api.createCategory(fileName, content, message);
      successMsg.set(`已创建分类 ${fileName}`);
      showNewDialog = false;
      await loadAll();
      selectedFileName = fileName;
      await loadFileContent();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function handleSync() {
    try {
      await api.syncPublic("手动同步公共部分");
      successMsg.set("公共部分已重生成");
      await loadAll();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  // AI 生成的代码应用到工作区
  async function handleApplyCode(code: string, category: string | null) {
    if (category) {
      // 有分类名 → 作为新分类文件或追加到现有
      const fileName = `${category}.ps1`;
      const existing = categories.find((c) => c.file_name === fileName);
      if (existing) {
        // 追加到现有文件
        if (!confirm(`将代码追加到现有分类 ${fileName}？`)) return;
        try {
          const old = await api.readCategoryFile(fileName);
          const newContent = old.trimEnd() + "\n\n" + code.trimEnd() + "\n";
          await api.updateCategoryFile(fileName, newContent, `AI 生成追加到 ${fileName}`);
          successMsg.set(`已追加到 ${fileName}`);
          await loadAll();
          selectedFileName = fileName;
          await loadFileContent();
          tab = "manage";
        } catch (e) {
          errorMsg.set(String(e));
        }
      } else {
        // 新分类
        try {
          await api.createCategory(fileName, code, `AI 生成新分类 ${category}`);
          successMsg.set(`已创建分类 ${fileName}`);
          await loadAll();
          selectedFileName = fileName;
          await loadFileContent();
          tab = "manage";
        } catch (e) {
          errorMsg.set(String(e));
        }
      }
    } else {
      // 无分类 → 让用户选择目标文件
      errorMsg.set("AI 生成的代码未含 @DST-Category 块，请在管理页手动添加。");
      tab = "manage";
    }
  }

  function fmtTime(secs: number): string {
    return new Date(secs * 1000).toLocaleString("zh-CN");
  }
  function shortOid(oid: string): string {
    return oid.slice(0, 8);
  }
</script>

<main class="h-screen flex flex-col">
  <header class="px-5 py-3 bg-slate-900/80 border-b border-slate-700 flex items-center justify-between">
    <div>
      <h1 class="text-lg font-bold text-cyan-300">DevShellTools Studio</h1>
      <p class="text-xs text-slate-500">模板 v1.0.5 · M3 AI 集成</p>
    </div>
    <nav class="flex gap-1">
      <button
        class="px-3 py-1 text-xs rounded {tab === 'manage' ? 'bg-cyan-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}"
        onclick={() => (tab = "manage")}>管理</button
      >
      <button
        class="px-3 py-1 text-xs rounded {tab === 'chat' ? 'bg-cyan-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}"
        onclick={() => (tab = "chat")}>AI 助手 {aiReady ? "" : "(未配置)"}</button
      >
      <button
        class="px-3 py-1 text-xs rounded {tab === 'settings' ? 'bg-cyan-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}"
        onclick={() => (tab = "settings")}>设置</button
      >
    </nav>
  </header>

  {#if $errorMsg}
    <div
      class="px-4 py-2 bg-red-900/50 border-b border-red-700 text-red-200 text-sm flex justify-between">
      <span>{$errorMsg}</span>
      <button class="ml-3 text-red-300 hover:text-red-100" onclick={clearMessages}>×</button>
    </div>
  {/if}
  {#if $successMsg}
    <div
      class="px-4 py-2 bg-green-900/50 border-b border-green-700 text-green-200 text-sm flex justify-between">
      <span>{$successMsg}</span>
      <button class="ml-3 text-green-300 hover:text-green-100" onclick={clearMessages}>×</button>
    </div>
  {/if}

  {#if $loading && !$workspace}
    <div class="flex-1 flex items-center justify-center text-slate-400">加载中…</div>
  {:else if !$workspace?.initialized}
    <div class="flex-1 flex items-center justify-center p-6">
      <div class="bg-slate-800/60 rounded-lg p-6 border border-slate-700 max-w-md text-center">
        <h2 class="text-lg font-semibold text-amber-300 mb-2">首次使用</h2>
        <p class="text-sm text-slate-300 mb-1">未检测到工作区。</p>
        <p class="text-xs text-slate-400 mb-4">
          将在 <code class="text-cyan-300">{$workspace?.root ?? ""}</code> 初始化 DevShellTools 1.0.5 模板。
        </p>
        <button
          class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded text-sm"
          onclick={handleInit}
          disabled={$loading}>
          {$loading ? "初始化中…" : "初始化工作区"}
        </button>
      </div>
    </div>
  {:else if tab === "manage"}
    <div class="flex-1 flex overflow-hidden">
      <CategoryList {categories} {selectedFileName} onSelect={onSelect} />
      <CategoryEditor
        category={selectedCategory}
        {fileContent}
        onSave={handleSave}
        onDelete={handleDelete} />
      <aside class="w-64 shrink-0 bg-slate-900/60 border-l border-slate-700 overflow-y-auto p-3">
        <h3 class="text-xs font-semibold text-slate-400 mb-2">一致性校验</h3>
        {#if consistency}
          <div
            class="p-2 rounded text-xs mb-3 {consistency.ok
              ? 'bg-green-900/40 border border-green-700 text-green-200'
              : 'bg-red-900/40 border border-red-700 text-red-200'}">
            {consistency.ok ? "✓ 通过" : "✗ 不一致"}
          </div>
          {#if consistency.errors.length > 0}
            <div class="mb-2">
              <div class="text-xs text-red-400 mb-1">错误：</div>
              <ul class="text-xs text-red-200 space-y-0.5">
                {#each consistency.errors as e}<li>· {e}</li>{/each}
              </ul>
            </div>
          {/if}
          {#if consistency.warnings.length > 0}
            <div class="mb-2">
              <div class="text-xs text-amber-400 mb-1">警告：</div>
              <ul class="text-xs text-amber-200 space-y-0.5">
                {#each consistency.warnings as w}<li>· {w}</li>{/each}
              </ul>
            </div>
          {/if}
          <div class="text-xs text-slate-500">
            实际 {consistency.actual_functions.length} · psd1 {consistency.psd1_exports.length} · psm1
            {consistency.psm1_exports.length}
          </div>
        {/if}

        <h3 class="text-xs font-semibold text-slate-400 mt-4 mb-2">Git 快照</h3>
        {#if logsLoading}
          <p class="text-xs text-slate-500">加载中…</p>
        {:else if commits.length === 0}
          <p class="text-xs text-slate-500">暂无</p>
        {:else}
          <ul class="space-y-1.5 text-xs">
            {#each commits as c}
              <li class="border-l-2 border-slate-600 pl-2">
                <div class="text-slate-500 font-mono">{shortOid(c.oid)} · {fmtTime(c.time)}</div>
                <div class="text-slate-300">{c.message}</div>
              </li>
            {/each}
          </ul>
        {/if}

        <div class="mt-4 flex gap-2">
          <button class="px-2 py-1 text-xs bg-slate-700 hover:bg-slate-600 rounded" onclick={handleSync}>
            同步公共部分
          </button>
          <button
            class="px-2 py-1 text-xs bg-cyan-600 hover:bg-cyan-500 rounded"
            onclick={() => (showNewDialog = true)}>新建分类</button
          >
        </div>
      </aside>
    </div>
  {:else if tab === "chat"}
    <div class="flex-1 overflow-hidden">
      {#if aiReady}
        <ChatPanel onApplyCode={handleApplyCode} onOpenSettings={() => (tab = "settings")} />
      {:else}
        <div class="h-full flex items-center justify-center p-6">
          <div class="text-center">
            <p class="text-sm text-amber-300 mb-3">AI 未配置</p>
            <p class="text-xs text-slate-400 mb-4">请先在设置页配置 API Key 和模型。</p>
            <button
              class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 rounded text-sm"
              onclick={() => (tab = "settings")}>前往设置</button
            >
          </div>
        </div>
      {/if}
    </div>
  {:else if tab === "settings"}
    <div class="flex-1 overflow-y-auto">
      <AiSettings onClose={() => (tab = "manage")} />
    </div>
  {/if}

  {#if showNewDialog}
    <NewCategoryDialog onCreate={handleCreate} onCancel={() => (showNewDialog = false)} />
  {/if}
</main>