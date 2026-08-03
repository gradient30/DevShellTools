<script lang="ts">
  import type { CategoryInfo, SafetyReport } from "../api";
  import { api } from "../api";

  let {
    category,
    fileContent,
    onSave,
    onDelete
  }: {
    category: CategoryInfo | null;
    fileContent: string;
    onSave: (content: string, message: string) => void;
    onDelete: (fileName: string) => void;
  } = $props();

  let editing = $state(false);
  let draft = $state("");
  let commitMsg = $state("");
  let syntaxOk = $state<boolean | null>(null);
  let syntaxErr = $state<string | null>(null);
  let safetyReport = $state<SafetyReport | null>(null);

  function startEdit() {
    draft = fileContent;
    editing = true;
    syntaxOk = null;
    syntaxErr = null;
    safetyReport = null;
  }

  function cancelEdit() {
    editing = false;
    draft = "";
    commitMsg = "";
  }

  async function validate() {
    syntaxOk = null;
    syntaxErr = null;
    safetyReport = null;
    try {
      await api.validatePsSyntax(draft);
      syntaxOk = true;
      const r = await api.safetyCheck(draft);
      safetyReport = r;
    } catch (e) {
      syntaxOk = false;
      syntaxErr = String(e);
    }
  }

  function save() {
    if (!commitMsg.trim()) {
      commitMsg = `更新 ${category?.file_name ?? ""}`;
    }
    onSave(draft, commitMsg);
    editing = false;
  }

  let canSave = $derived(syntaxOk === true && (safetyReport?.ok ?? false));
</script>

<section class="flex-1 overflow-y-auto p-4">
  {#if !category}
    <div class="text-slate-500 text-sm">请从左侧选择一个分类。</div>
  {:else if editing}
    <div class="mb-3 flex items-center justify-between">
      <h2 class="text-lg font-semibold text-cyan-300">编辑 {category.file_name}</h2>
      <div class="flex gap-2">
        <button class="px-3 py-1 text-sm bg-slate-700 hover:bg-slate-600 rounded" onclick={validate}>
          校验
        </button>
        <button
          class="px-3 py-1 text-sm bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50"
          onclick={save}
          disabled={!canSave}>保存</button
        >
        <button class="px-3 py-1 text-sm bg-slate-700 hover:bg-slate-600 rounded" onclick={cancelEdit}>
          取消
        </button>
      </div>
    </div>

    <input
      type="text"
      bind:value={commitMsg}
      placeholder="提交说明（可选）"
      class="w-full mb-2 px-3 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded text-slate-200" />

    <textarea
      bind:value={draft}
      rows="22"
      class="w-full px-3 py-2 text-xs font-mono bg-slate-950 border border-slate-700 rounded text-slate-200 resize-y"
      spellcheck="false"></textarea>

    {#if syntaxOk === false}
      <div class="mt-2 p-2 text-xs bg-red-900/40 border border-red-700 text-red-200 rounded">
        语法错误：{syntaxErr}
      </div>
    {/if}
    {#if syntaxOk === true}
      <div class="mt-2 p-2 text-xs bg-green-900/40 border border-green-700 text-green-200 rounded">
        语法校验通过
      </div>
    {/if}
    {#if safetyReport && !safetyReport.ok}
      <div class="mt-2 p-2 text-xs bg-red-900/40 border border-red-700 text-red-200 rounded">
        安全拦截：{safetyReport.violations.join("；")}
      </div>
    {/if}
    {#if safetyReport?.ok}
      <div class="mt-2 p-2 text-xs bg-green-900/40 border border-green-700 text-green-200 rounded">
        安全检查通过
      </div>
    {/if}
  {:else}
    <div class="mb-3 flex items-center justify-between">
      <div>
        <h2 class="text-lg font-semibold text-cyan-300">{category.category.title}</h2>
        <p class="text-xs text-slate-400 mt-0.5">{category.category.description}</p>
      </div>
      <div class="flex gap-2">
        <button class="px-3 py-1 text-sm bg-cyan-600 hover:bg-cyan-500 rounded" onclick={startEdit}>
          编辑
        </button>
        <button
          class="px-3 py-1 text-sm bg-red-700 hover:bg-red-600 rounded"
          onclick={() => onDelete(category.file_name)}>删除</button
        >
      </div>
    </div>

    <div class="mb-4 grid grid-cols-2 gap-2 text-xs">
      <div class="bg-slate-800/50 rounded p-2">
        <span class="text-slate-400">关键字：</span>
        <code class="text-cyan-300">{category.category.name}</code>
      </div>
      <div class="bg-slate-800/50 rounded p-2">
        <span class="text-slate-400">别名：</span>
        <span class="text-slate-200"
          >{category.category.aliases.length > 0 ? category.category.aliases.join(", ") : "无"}</span
        >
      </div>
    </div>

    <h3 class="text-sm font-semibold text-slate-300 mb-2">命令列表</h3>
    <table class="w-full text-xs">
      <thead class="text-slate-400 border-b border-slate-700">
        <tr>
          <th class="text-left py-1.5 pr-4">命令</th>
          <th class="text-left py-1.5 pr-4">说明</th>
          <th class="text-left py-1.5">示例</th>
        </tr>
      </thead>
      <tbody>
        {#each category.functions as f (f.name)}
          <tr class="border-b border-slate-800">
            <td class="py-1.5 pr-4 font-mono text-cyan-200">{f.name}</td>
            <td class="py-1.5 pr-4 text-slate-300">{f.synopsis || "(无说明)"}</td>
            <td class="py-1.5 font-mono text-slate-400">{f.first_example}</td>
          </tr>
        {/each}
      </tbody>
    </table>

    <h3 class="text-sm font-semibold text-slate-300 mt-4 mb-2">源码</h3>
    <pre class="text-xs font-mono bg-slate-950 border border-slate-700 rounded p-3 overflow-x-auto text-slate-300 max-h-64">{fileContent}</pre>
  {/if}
</section>