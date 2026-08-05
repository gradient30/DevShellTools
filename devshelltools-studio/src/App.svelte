<script lang="ts">
  import { onMount } from "svelte";
  import {
    workspace,
    loading,
    errorMsg,
    successMsg,
    initProgress,
    refresh,
    init,
    clearMessages
  } from "./lib/stores/workspace";
  import { showToast } from "./lib/stores/toast";
  import { api, type CategoryInfo, type ConsistencyReport, type InstallStatus, type MigrationCheck, type PsFunction, type Webview2Status } from "./lib/api";
  import CategoryList from "./lib/components/CategoryList.svelte";
  import CategoryEditor from "./lib/components/CategoryEditor.svelte";
  import NewCategoryDialog from "./lib/components/NewCategoryDialog.svelte";
  import ChatPanel from "./lib/components/ChatPanel.svelte";
  import AiSettings from "./lib/components/AiSettings.svelte";
  import ToolsPage from "./lib/components/ToolsPage.svelte";
  import ToastHost from "./lib/components/ToastHost.svelte";

  type Tab = "manage" | "chat" | "settings" | "tools";
  let tab = $state<Tab>("manage");

  let categories = $state<CategoryInfo[]>([]);
  let categoriesLoading = $state(false);
  let categoriesLoadMsg = $state("");
  let categoriesLoaded = $state(false);
  let selectedFileName = $state<string | null>(null);
  let selectedCategory = $derived(categories.find((c) => c.file_name === selectedFileName) ?? null);
  let fileContent = $state("");
  let consistency = $state<ConsistencyReport | null>(null);
  let showNewDialog = $state(false);
  let aiReady = $state(false);
  let aiReadyLoading = $state(false);
  let aiReadyChecked = $state(false);
  let installStatus = $state<InstallStatus | null>(null);
  let installBusy = $state(false);
  let aiPrompt = $state("");

  let toolsMigration = $state<MigrationCheck | null>(null);
  let toolsWebview2 = $state<Webview2Status | null>(null);
  let toolsLogFiles = $state<string[]>([]);
  let toolsLoaded = $state(false);

  onMount(async () => {
    await refresh();
    if ($workspace?.initialized) {
      void loadInstallStatus();
      void loadCategories();
      void loadAiReady();
    }
    // AI 配置变更后即时刷新 AI 助手状态
    window.addEventListener("ai-config-changed", () => {
      void loadAiReady();
    });
  });

  $effect(() => {
    if (tab === "tools" && $workspace?.initialized && !toolsLoaded) {
      loadToolsData();
    }
  });

  async function loadInstallStatus() {
    try {
      installStatus = await api.installStatus();
    } catch (e) {
      installStatus = null;
      showToast(`安装状态检测失败：${String(e)}`, "error", 5000);
    }
  }

  async function loadCategories() {
    if (categoriesLoading) return;
    categoriesLoading = true;
    // 与「初始化成功」分阶段：成功条改为后续步骤提示，避免矛盾观感
    if ($successMsg === "工作区初始化成功。") {
      successMsg.set("工作区已就绪，正在解析分类元数据…");
    }
    categoriesLoadMsg = "正在解析分类（PowerShell 元数据）…";
    try {
      const result = await api.listCategories();
      categories = result.categories;
      categoriesLoaded = true;
      categoriesLoadMsg = result.cached ? "已从缓存加载分类" : "分类解析完成";
      if ($successMsg?.includes("正在解析分类")) {
        successMsg.set(null);
      }
      if (!result.cached) {
        showToast("分类信息已更新", "success", 2500);
      }
      if (!selectedFileName && categories.length > 0) {
        selectedFileName = categories[0].file_name;
        await loadFileContent();
      } else if (selectedFileName) {
        await loadFileContent();
      }
    } catch (e) {
      console.error(e);
      showToast(String(e), "error");
      categoriesLoaded = true;
      categoriesLoadMsg = "加载失败";
    } finally {
      categoriesLoading = false;
      setTimeout(() => {
        categoriesLoadMsg = "";
      }, 2000);
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

  async function loadConsistency() {
    try {
      consistency = await api.consistencyCheck();
    } catch (e) {
      console.error(e);
    }
  }

  async function loadAiReady() {
    aiReadyLoading = true;
    try {
      aiReady = await api.aiReady();
    } catch {
      aiReady = false;
    } finally {
      aiReadyLoading = false;
      aiReadyChecked = true;
    }
  }

  async function loadManageSidebar() {
    await loadConsistency();
  }

  async function loadToolsData() {
    try {
      [toolsMigration, toolsWebview2, toolsLogFiles] = await Promise.all([
        api.checkMigration(),
        api.webview2Status(),
        api.listLogs()
      ]);
      toolsLoaded = true;
    } catch (e) {
      console.error(e);
      toolsLoaded = true;
    }
  }

  $effect(() => {
    // 等分类加载完成后再做一致性校验，避免与 parse_public_batch 叠加重负载导致界面卡顿
    if (
      tab === "manage" &&
      $workspace?.initialized &&
      categoriesLoaded &&
      !categoriesLoading &&
      !consistency
    ) {
      void loadManageSidebar();
    }
  });

  async function handleInit() {
    await init();
    if ($workspace?.initialized) {
      categoriesLoaded = false;
      consistency = null;
      await loadInstallStatus();
      await loadCategories();
      aiReadyChecked = false;
      void loadAiReady();
      toolsLoaded = false;
    }
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
      await loadCategories();
      if (tab === "manage") await loadManageSidebar();
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
      await loadCategories();
      if (tab === "manage") await loadManageSidebar();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function handleCreate(fileName: string, content: string, message: string) {
    try {
      await api.createCategory(fileName, content, message);
      successMsg.set(`已创建分类 ${fileName}`);
      showNewDialog = false;
      await loadCategories();
      selectedFileName = fileName;
      if (tab === "manage") await loadManageSidebar();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function handleSync() {
    try {
      await api.syncPublic("手动同步公共部分");
      successMsg.set("公共部分已重生成");
      await loadCategories();
      await loadManageSidebar();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  async function handleApplyCode(code: string, fileName: string) {
    try {
      const applied = await api.applyAiCode(fileName, code, `AI 插入到 ${fileName}`);
      successMsg.set(`已插入：${applied.join(", ")}`);
      await loadCategories();
      selectedFileName = fileName;
      tab = "manage";
      await loadManageSidebar();
    } catch (e) {
      errorMsg.set(String(e));
    }
  }

  function handleAiGenerate(func: PsFunction | null) {
    const cat = selectedCategory;
    if (!cat) return;
    const base = func
      ? `请为 DevShellTools 分类「${cat.category.title}」优化/实现命令 ${func.name}。当前说明：${func.synopsis}。示例：${func.first_example || func.name}。`
      : `请为 DevShellTools 分类「${cat.category.title}」生成一个新的 PowerShell 快捷命令。`;
    aiPrompt = `${base}\n已有命令：${cat.functions.map((f) => f.name).join(", ")}。`;
    tab = "chat";
  }

  async function handleInstallToggle() {
    if (installStatus?.installed) {
      if (!confirm("确认卸载？将移除 Profile 中的 Import-Module 和模块副本。")) {
        return;
      }
      installBusy = true;
      try {
        const result = await api.uninstallModule();
        installStatus = result.status;
        showToast(result.message, result.verified ? "success" : "info", 6000);
      } catch (e) {
        showToast(String(e), "error", 6000);
      } finally {
        installBusy = false;
      }
    } else {
      installBusy = true;
      try {
        const result = await api.installModule();
        installStatus = result.status;
        showToast(result.message, result.verified ? "success" : "info", 6000);
      } catch (e) {
        showToast(String(e), "error", 6000);
      } finally {
        installBusy = false;
      }
    }
  }

    </script>

<main class="h-screen flex flex-col bg-slate-950">
  <ToastHost />
  <header class="px-5 py-3 bg-slate-900/80 border-b border-slate-700 flex items-center justify-between">
    <div>
      <h1 class="text-lg font-bold text-cyan-300">DevShellTools Studio</h1>
      <p class="text-xs text-slate-500">模板 v1.0.5 · M6</p>
    </div>
    <nav class="flex gap-1 items-center">
      <button
        class="px-3 py-1 text-xs rounded {tab === 'manage' ? 'bg-cyan-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}"
        onclick={() => (tab = "manage")}>管理</button
      >
      <button
        class="px-3 py-1 text-xs rounded {tab === 'chat' ? 'bg-cyan-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}"
        onclick={() => (tab = "chat")}>AI 助手 {aiReadyLoading ? "…" : aiReady ? "" : "(未配置)"}</button
      >
      <button
        class="px-3 py-1 text-xs rounded {tab === 'tools' ? 'bg-cyan-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}"
        onclick={() => (tab = "tools")}>工具箱</button
      >
      <button
        class="px-3 py-1 text-xs rounded {tab === 'settings' ? 'bg-cyan-600 text-white' : 'bg-slate-700 text-slate-300 hover:bg-slate-600'}"
        onclick={() => (tab = "settings")}>设置</button
      >
      {#if $workspace?.initialized || installStatus?.installed}
        <span class="w-px h-5 bg-slate-700 mx-1"></span>
        <button
          class="px-3 py-1 text-xs rounded disabled:opacity-50 {installStatus?.installed
            ? 'bg-amber-700 hover:bg-amber-600 text-white'
            : 'bg-emerald-700 hover:bg-emerald-600 text-white'}"
          onclick={handleInstallToggle}
          disabled={installBusy || installStatus === null}
          title={installStatus
            ? `模块:${installStatus.ps51_module_present || installStatus.ps7_module_present ? "有" : "无"} · Profile:${installStatus.profile_configured ? "已配置" : "未配置"}`
            : "正在检测安装状态…"}>
          {installBusy
            ? "处理中…"
            : installStatus === null
              ? "检测中…"
              : installStatus.installed
                ? "卸载"
                : "安装"}
        </button>
      {/if}
    </nav>
  </header>

  {#if $errorMsg}
    <div class="px-4 py-2 bg-red-900/50 border-b border-red-700 text-red-200 text-sm flex justify-between">
      <span>{$errorMsg}</span>
      <button class="ml-3 text-red-300 hover:text-red-100" onclick={clearMessages}>×</button>
    </div>
  {/if}
  {#if $successMsg}
    <div class="px-4 py-2 bg-green-900/50 border-b border-green-700 text-green-200 text-sm flex justify-between">
      <span>{$successMsg}</span>
      <button class="ml-3 text-green-300 hover:text-green-100" onclick={clearMessages}>×</button>
    </div>
  {/if}

  {#if $loading && !$workspace}
    <div class="flex-1 flex flex-col items-center justify-center text-slate-400 gap-3">
      <div class="h-8 w-8 border-2 border-cyan-500/30 border-t-cyan-400 rounded-full animate-spin"></div>
      <p class="text-sm">正在连接工作区…</p>
    </div>
  {:else if !$workspace?.initialized}
    <div class="flex-1 flex items-center justify-center p-6">
      <div class="bg-slate-800/60 rounded-lg p-6 border border-slate-700 max-w-md w-full text-center">
        <h2 class="text-lg font-semibold text-amber-300 mb-2">首次使用</h2>
        <p class="text-sm text-slate-300 mb-1">未检测到工作区。</p>
        <p class="text-xs text-slate-400 mb-4">
          将在 <code class="text-cyan-300">{$workspace?.root ?? ""}</code> 初始化 DevShellTools 1.0.5 模板。
        </p>
        {#if $initProgress}
          <div class="mb-4 text-left">
            <div class="flex justify-between text-xs text-slate-400 mb-1">
              <span>{$initProgress.label}</span>
              <span>{$initProgress.percent}%</span>
            </div>
            <div class="h-2 bg-slate-900 rounded overflow-hidden">
              <div class="h-full bg-cyan-500 transition-all duration-300" style="width: {$initProgress.percent}%"></div>
            </div>
          </div>
        {/if}
        <button
          class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded text-sm disabled:opacity-50"
          onclick={handleInit}
          disabled={$loading}>
          {$loading ? "初始化中…" : "初始化工作区"}
        </button>
      </div>
    </div>
  {:else}
    <div class="flex-1 flex flex-col min-h-0 overflow-hidden">
    {#if categoriesLoading}
      <div class="px-4 py-2 bg-cyan-950/70 border-b border-cyan-800/80 text-cyan-100 text-sm flex items-center gap-3 shrink-0">
        <div class="h-4 w-4 border-2 border-cyan-400/30 border-t-cyan-400 rounded-full animate-spin shrink-0"></div>
        <span>{categoriesLoadMsg || "正在加载分类信息…"}</span>
        <span class="text-xs text-cyan-400/80 ml-auto">首次约 5–10 秒，之后从缓存秒开；加载期间请稍候</span>
      </div>
    {/if}
    <div class="flex-1 overflow-hidden relative min-h-0">
      {#if categoriesLoading}
        <div
          class="absolute inset-0 z-20 bg-slate-950/40 backdrop-blur-[1px] flex items-center justify-center pointer-events-auto"
          aria-busy="true">
          <div class="bg-slate-900 border border-slate-700 rounded-lg px-5 py-4 text-sm text-slate-200 shadow-lg max-w-sm text-center">
            <div class="mx-auto mb-3 h-6 w-6 border-2 border-cyan-400/30 border-t-cyan-400 rounded-full animate-spin"></div>
            <p>{categoriesLoadMsg || "正在解析分类元数据…"}</p>
            <p class="text-xs text-slate-500 mt-2">后台调用 PowerShell AST，界面暂时锁定以免误点</p>
          </div>
        </div>
      {/if}
      <div class="absolute inset-0 flex overflow-hidden" class:hidden={tab !== "manage"} aria-hidden={tab !== "manage"}>
        <CategoryList {categories} {selectedFileName} loading={categoriesLoading} onSelect={onSelect} />
        <CategoryEditor
          category={selectedCategory}
          {fileContent}
          onSave={handleSave}
          onDelete={handleDelete}
          onChanged={loadCategories}
          onAiGenerate={handleAiGenerate} />
        <aside class="w-64 shrink-0 bg-slate-900/60 border-l border-slate-700 overflow-y-auto p-3">
          <h3 class="text-xs font-semibold text-slate-400 mb-2">一致性校验</h3>
          {#if categoriesLoading || !consistency}
            <div class="h-16 bg-slate-800/40 rounded animate-pulse"></div>
            {#if categoriesLoading}
              <p class="text-xs text-slate-500 mt-2">等待分类解析完成…</p>
            {/if}
          {:else if consistency}
            <div
              class="p-2 rounded text-xs mb-3 {consistency.ok
                ? 'bg-green-900/40 border border-green-700 text-green-200'
                : 'bg-red-900/40 border border-red-700 text-red-200'}">
              {consistency.ok ? "✓ 通过" : "✗ 不一致"}
            </div>
            {#if consistency.errors.length > 0}
              <ul class="text-xs text-red-200 space-y-0.5 mb-2">
                {#each consistency.errors as e}<li>· {e}</li>{/each}
              </ul>
            {/if}
            <div class="text-xs text-slate-500 mb-3">
              实际 {consistency.actual_functions.length} · psd1 {consistency.psd1_exports.length}
            </div>
          {/if}

          <div class="mt-4 flex gap-2">
            <button class="px-2 py-1 text-xs bg-slate-700 hover:bg-slate-600 rounded" onclick={handleSync}>同步公共部分</button>
            <button class="px-2 py-1 text-xs bg-cyan-600 hover:bg-cyan-500 rounded" onclick={() => (showNewDialog = true)}>新建分类</button>
          </div>
        </aside>
      </div>

      <div class="absolute inset-0 overflow-hidden" class:hidden={tab !== "chat"} aria-hidden={tab !== "chat"}>
        {#if aiReadyLoading}
          <div class="h-full flex items-center justify-center text-slate-400 text-sm">正在检查 AI 配置…</div>
        {:else if aiReady}
          <ChatPanel
            {categories}
            initialPrompt={aiPrompt}
            onApplyCode={handleApplyCode}
            onOpenSettings={() => (tab = "settings")} />
        {:else}
          <div class="h-full flex items-center justify-center p-6">
            <div class="text-center">
              <p class="text-sm text-amber-300 mb-3">AI 未配置</p>
              <button class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 rounded text-sm" onclick={() => (tab = "settings")}>前往设置</button>
            </div>
          </div>
        {/if}
      </div>

      <div class="absolute inset-0 overflow-y-auto" class:hidden={tab !== "settings"} aria-hidden={tab !== "settings"}>
        <AiSettings />
      </div>

      <div class="absolute inset-0 overflow-y-auto" class:hidden={tab !== "tools"} aria-hidden={tab !== "tools"}>
        <ToolsPage
          migration={toolsMigration}
          webview2={toolsWebview2}
          logFiles={toolsLogFiles}
          loaded={toolsLoaded}
          onRefresh={loadToolsData} />
      </div>
    </div>
    </div>
  {/if}

  {#if showNewDialog}
    <NewCategoryDialog onCreate={handleCreate} onCancel={() => (showNewDialog = false)} />
  {/if}
</main>
