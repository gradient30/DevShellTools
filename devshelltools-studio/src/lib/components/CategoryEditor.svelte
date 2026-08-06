<script lang="ts">
  import type { CategoryInfo, SafetyReport } from "../api";
  import { api } from "../api";
  import CommandTable from "./CommandTable.svelte";

  let {
    category,
    fileContent,
    onSave,
    onDelete,
    onChanged,
    onAiGenerate
  }: {
    category: CategoryInfo | null;
    fileContent: string;
    onSave: (content: string, message: string) => void;
    onDelete: (fileName: string) => void;
    onChanged: () => void | Promise<void>;
    onAiGenerate: (func: { name: string; synopsis: string; first_example: string } | null) => void;
  } = $props();

  let view = $state<"overview" | "source">("overview");
  let draft = $state("");
  let syntaxOk = $state<boolean | null>(null);
  let syntaxErr = $state<string | null>(null);
  let safetyReport = $state<SafetyReport | null>(null);
  let dirty = $state(false);

  // 行号计算
  let draftLines = $derived(draft.split("\n"));
  let lineCount = $derived(draftLines.length);

  function startEdit() {
    draft = fileContent;
    view = "source";
    syntaxOk = null;
    syntaxErr = null;
    safetyReport = null;
    dirty = false;
  }

  function cancelEdit() {
    if (dirty && !confirm("有未保存的修改，确认放弃？")) return;
    view = "overview";
    draft = "";
    dirty = false;
  }

  async function validate() {
    syntaxOk = null;
    syntaxErr = null;
    safetyReport = null;
    try {
      await api.validatePsSyntax(draft);
      syntaxOk = true;
      safetyReport = await api.safetyCheck(draft);
    } catch (e) {
      syntaxOk = false;
      syntaxErr = String(e);
    }
  }

  function save() {
    onSave(draft, `更新 ${category?.file_name ?? ""}`);
    view = "overview";
    dirty = false;
  }

  function onInput() {
    dirty = true;
    syntaxOk = null;
    safetyReport = null;
  }

  let canSave = $derived(syntaxOk === true && (safetyReport?.ok ?? false));
  let funcCount = $derived(
    category?.functions.filter((f) => /^[a-z]/.test(f.name)).length ?? 0
  );
</script>

<section class="flex-1 flex flex-col overflow-hidden">
  {#if !category}
    <div class="flex-1 flex items-center justify-center text-dst-fg-muted text-sm">
      <div class="text-center">
        <div class="text-4xl mb-3 opacity-30">📋</div>
        <p>请从左侧选择一个分类</p>
      </div>
    </div>
  {:else if view === "source"}
    <!-- 源码编辑视图 -->
    <div class="px-4 py-2.5 border-b border-dst-border flex items-center justify-between bg-dst-surface">
      <div class="flex items-center gap-3">
        <button class="text-dst-fg-muted hover:text-dst-fg text-sm" onclick={() => view = "overview"}>
          ← 返回概览
        </button>
        <span class="text-dst-fg-subtle">|</span>
        <h2 class="text-sm font-mono text-dst-accent">{category.file_name}</h2>
        {#if dirty}<span class="text-xs text-dst-warning">● 未保存</span>{/if}
      </div>
      <div class="flex items-center gap-2">
        <button class="px-3 py-1 text-xs bg-dst-muted hover:bg-dst-muted rounded transition-colors" onclick={validate}>
          校验
        </button>
        <button
          class="px-3 py-1 text-xs bg-dst-accent text-dst-accent-fg hover:bg-dst-accent-hover rounded transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          onclick={save}
          disabled={!canSave}>
          保存
        </button>
        <button class="px-3 py-1 text-xs bg-dst-muted hover:bg-dst-muted rounded transition-colors" onclick={cancelEdit}>
          取消
        </button>
      </div>
    </div>

    <!-- 校验状态条 -->
    {#if syntaxOk === true || syntaxOk === false || (safetyReport && !safetyReport.ok)}
      <div class="px-4 py-1.5 border-b border-dst-border space-y-1">
        {#if syntaxOk === true}
          <div class="flex items-center gap-1.5 text-xs text-dst-success">
            <span>✓</span> 语法校验通过 · {lineCount} 行
          </div>
        {:else if syntaxOk === false}
          <div class="text-xs text-dst-danger">✗ 语法错误：{syntaxErr}</div>
        {/if}
        {#if safetyReport && !safetyReport.ok}
          <div class="text-xs text-dst-danger">✗ 安全拦截：{safetyReport.violations.join("；")}</div>
        {/if}
        {#if safetyReport?.ok}
          <div class="text-xs text-dst-success">✓ 安全检查通过</div>
        {/if}
      </div>
    {/if}

    <!-- 代码编辑区：行号 + textarea -->
    <div class="flex-1 overflow-auto bg-dst-bg">
      <div class="flex min-h-full">
        <!-- 行号栏 -->
        <div class="select-none py-3 px-2 text-right text-xs font-mono text-dst-fg-subtle bg-dst-surface/30 border-r border-dst-border leading-5 shrink-0" style="min-width: 3rem;">
          {#each draftLines as _, i}
            <div>{i + 1}</div>
          {/each}
        </div>
        <!-- 代码区 -->
        <textarea
          bind:value={draft}
          oninput={onInput}
          rows="30"
          class="flex-1 py-3 px-3 text-xs font-mono bg-dst-bg text-dst-fg leading-5 resize-none border-0 outline-none whitespace-pre"
          spellcheck="false"
          style="tab-size: 4;"></textarea>
      </div>
    </div>

    <!-- 底部状态栏 -->
    <div class="px-4 py-1.5 border-t border-dst-border bg-dst-surface flex items-center justify-between text-xs text-dst-fg-muted">
      <span>{lineCount} 行 · {draft.length} 字符</span>
      <span class="font-mono">PowerShell · UTF-8</span>
    </div>
  {:else}
    <!-- 概览视图 -->
    <div class="flex-1 overflow-y-auto p-5">
      <!-- 分类标题区 -->
      <div class="mb-5 flex items-start justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-dst-elevated border border-dst-accent flex items-center justify-center text-dst-accent text-lg shrink-0">
            📦
          </div>
          <div>
            <h2 class="text-lg font-semibold text-dst-accent">{category.category.title}</h2>
            <p class="text-xs text-dst-fg-muted mt-0.5">{category.category.description}</p>
          </div>
        </div>
        <div class="flex gap-2">
          <button class="px-3 py-1.5 text-sm bg-dst-muted hover:bg-dst-muted rounded transition-colors" onclick={startEdit}>
            ✏️ 编辑源码
          </button>
          <button
            class="px-3 py-1.5 text-sm bg-dst-btn-danger text-dst-btn-danger-fg hover:opacity-90 rounded transition-colors"
            onclick={() => onDelete(category.file_name)}>
            🗑 删除
          </button>
        </div>
      </div>

      <!-- 元数据卡片 -->
      <div class="mb-5 grid grid-cols-3 gap-3">
        <div class="bg-dst-elevated rounded-lg p-3 border border-dst-border">
          <div class="text-xs text-dst-fg-muted mb-1">关键字</div>
          <code class="text-sm text-dst-accent font-mono">{category.category.name}</code>
        </div>
        <div class="bg-dst-elevated rounded-lg p-3 border border-dst-border">
          <div class="text-xs text-dst-fg-muted mb-1">别名</div>
          <span class="text-sm text-dst-fg">{category.category.aliases.length > 0 ? category.category.aliases.join(", ") : "无"}</span>
        </div>
        <div class="bg-dst-elevated rounded-lg p-3 border border-dst-border">
          <div class="text-xs text-dst-fg-muted mb-1">命令数</div>
          <span class="text-sm text-dst-fg">{funcCount}</span>
        </div>
      </div>

      <!-- 命令列表：只展示小写开头的公共命令（过滤 Assert-Git 等内部辅助） -->
      <CommandTable
        fileName={category.file_name}
        functions={category.functions.filter((f) => /^[a-z]/.test(f.name))}
        {onChanged}
        {onAiGenerate} />
    </div>
  {/if}
</section>