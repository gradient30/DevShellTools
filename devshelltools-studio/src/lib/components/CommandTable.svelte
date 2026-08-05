<script lang="ts">
  import type { PsFunction } from "../api";
  import { api } from "../api";
  import { showToast } from "../stores/toast";

  let {
    fileName,
    functions,
    onChanged,
    onAiGenerate
  }: {
    fileName: string;
    functions: PsFunction[];
    onChanged: () => void | Promise<void>;
    onAiGenerate: (func: PsFunction | null) => void;
  } = $props();

  let editingName = $state<string | null>(null);
  let draftName = $state("");
  let draftSynopsis = $state("");
  let draftExample = $state("");
  let busy = $state(false);
  let errMsg = $state<string | null>(null);
  let expandedName = $state<string | null>(null);

  function startAdd() {
    editingName = "__new__";
    draftName = "";
    draftSynopsis = "";
    draftExample = "";
    errMsg = null;
    expandedName = null;
  }

  function startEdit(f: PsFunction) {
    editingName = f.name;
    draftName = f.name;
    draftSynopsis = f.synopsis;
    draftExample = f.first_example || f.name;
    errMsg = null;
    expandedName = null;
  }

  function cancelEdit() {
    editingName = null;
    errMsg = null;
  }

  function toggleExpand(name: string) {
    expandedName = expandedName === name ? null : name;
  }

  async function save() {
    if (!draftName.trim() || !draftSynopsis.trim()) {
      errMsg = "命令名与说明不能为空";
      return;
    }
    busy = true;
    errMsg = null;
    const name = draftName.trim();
    try {
      await api.upsertFunction(
        fileName,
        name,
        draftSynopsis.trim(),
        draftExample.trim() || name,
        null,
        `更新命令 ${name}`
      );
      editingName = null;
      void onChanged();
      showToast(`已保存 ${name}`, "success", 2200);
    } catch (e) {
      errMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function remove(funcName: string) {
    if (!confirm(`确认删除命令 ${funcName}？`)) return;
    busy = true;
    errMsg = null;
    try {
      await api.deleteFunction(fileName, funcName, `删除命令 ${funcName}`);
      void onChanged();
      showToast(`已删除 ${funcName}`, "info", 2000);
    } catch (e) {
      errMsg = String(e);
    } finally {
      busy = false;
    }
  }

  async function test(funcName: string) {
    busy = true;
    try {
      const r = await api.testFunction(fileName, funcName);
      const msg = r.ok ? r.stdout.trim() || "(执行成功，无输出)" : r.stderr;
      showToast(`${funcName} · ${r.ok ? "通过" : "失败"}\n${msg}`, r.ok ? "success" : "error", 5000);
    } catch (e) {
      showToast(`${funcName} · 失败\n${String(e)}`, "error", 5000);
    } finally {
      busy = false;
    }
  }
</script>

<div>
  <!-- 标题栏 -->
  <div class="flex items-center justify-between mb-3">
    <h3 class="text-sm font-semibold text-slate-300 flex items-center gap-2">
      <span class="text-cyan-400">⚡</span> 命令列表
      <span class="text-xs text-slate-500 font-normal">({functions.length})</span>
    </h3>
    <button
      class="px-3 py-1 text-xs bg-cyan-600 hover:bg-cyan-500 rounded transition-colors disabled:opacity-50 flex items-center gap-1"
      onclick={startAdd}
      disabled={busy}>
      + 添加命令
    </button>
  </div>

  {#if errMsg}
    <div class="mb-3 p-2.5 text-xs bg-red-900/40 border border-red-700 text-red-200 rounded">{errMsg}</div>
  {/if}

  <!-- 新建/编辑表单 -->
  {#if editingName}
    <div class="mb-3 p-4 bg-slate-800/60 border border-cyan-700/50 rounded-lg space-y-3">
      <div class="text-xs text-cyan-300 font-semibold mb-1">
        {editingName === "__new__" ? "✨ 新建命令" : `编辑 ${editingName}`}
      </div>
      <div class="grid grid-cols-3 gap-3">
        <div>
          <label class="block text-xs text-slate-500 mb-1">命令名</label>
          <input
            bind:value={draftName}
            placeholder="如 gs"
            disabled={editingName !== "__new__"}
            class="w-full px-2.5 py-1.5 text-xs bg-slate-950 border border-slate-700 rounded font-mono text-cyan-200 focus:border-cyan-600 focus:outline-none disabled:opacity-60" />
        </div>
        <div class="col-span-2">
          <label class="block text-xs text-slate-500 mb-1">说明（SYNOPSIS）</label>
          <input
            bind:value={draftSynopsis}
            placeholder="如 查看 Git 状态"
            class="w-full px-2.5 py-1.5 text-xs bg-slate-950 border border-slate-700 rounded text-slate-200 focus:border-cyan-600 focus:outline-none" />
        </div>
      </div>
      <div>
        <label class="block text-xs text-slate-500 mb-1">示例（EXAMPLE）</label>
        <input
          bind:value={draftExample}
          placeholder="如 gs"
          class="w-full px-2.5 py-1.5 text-xs bg-slate-950 border border-slate-700 rounded font-mono text-slate-300 focus:border-cyan-600 focus:outline-none" />
      </div>
      <div class="flex gap-2 pt-1">
        <button class="px-3 py-1.5 text-xs bg-cyan-600 hover:bg-cyan-500 rounded transition-colors disabled:opacity-50" onclick={save} disabled={busy}>
          保存
        </button>
        <button class="px-3 py-1.5 text-xs bg-slate-700 hover:bg-slate-600 rounded transition-colors" onclick={cancelEdit}>
          取消
        </button>
      </div>
    </div>
  {/if}

  <!-- 命令卡片列表 -->
  {#if functions.length === 0 && !editingName}
    <div class="text-center py-8 text-slate-600 text-sm">
      暂无命令，点击"添加命令"创建
    </div>
  {:else}
    <div class="space-y-2">
      {#each functions as f (f.name)}
        <div class="bg-slate-800/40 border border-slate-700/50 rounded-lg overflow-hidden transition-all hover:border-slate-600">
          <!-- 命令行 -->
          <div class="flex items-center justify-between px-3 py-2.5 cursor-pointer hover:bg-slate-800/60" onclick={() => toggleExpand(f.name)}>
            <div class="flex items-center gap-3 min-w-0">
              <code class="text-sm font-mono text-cyan-300 font-medium shrink-0">{f.name}</code>
              <span class="text-xs text-slate-400 truncate">{f.synopsis || "(无说明)"}</span>
            </div>
            <div class="flex items-center gap-1 shrink-0" onclick={(e) => e.stopPropagation()}>
              <button class="px-2 py-0.5 text-xs text-cyan-400 hover:text-cyan-200 hover:bg-cyan-900/30 rounded transition-colors" onclick={() => startEdit(f)} disabled={busy}>
                编辑
              </button>
              <button class="px-2 py-0.5 text-xs text-emerald-400 hover:text-emerald-200 hover:bg-emerald-900/30 rounded transition-colors" onclick={() => test(f.name)} disabled={busy}>
                测试
              </button>
              <button
                class="px-2 py-0.5 text-xs text-amber-400 hover:text-amber-200 hover:bg-amber-900/30 rounded transition-colors"
                onclick={() => onAiGenerate(f)}
                disabled={busy}
                title="用 AI 检查当前命令：有问题给修复，无问题给优化/扩展建议">
                AI审阅
              </button>
              <button class="px-2 py-0.5 text-xs text-red-400 hover:text-red-200 hover:bg-red-900/30 rounded transition-colors" onclick={() => remove(f.name)} disabled={busy}>
                删
              </button>
              <span class="text-slate-600 text-xs ml-1">{expandedName === f.name ? "▾" : "▸"}</span>
            </div>
          </div>
          <!-- 展开详情 -->
          {#if expandedName === f.name}
            <div class="px-3 py-2.5 border-t border-slate-700/50 bg-slate-950/30">
              <div class="grid grid-cols-2 gap-2 text-xs">
                <div>
                  <span class="text-slate-500">示例：</span>
                  <code class="text-slate-300 font-mono">{f.first_example || f.name}</code>
                </div>
                <div>
                  <span class="text-slate-500">说明：</span>
                  <span class="text-slate-300">{f.synopsis || "(无)"}</span>
                </div>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>