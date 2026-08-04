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

  function startAdd() {
    editingName = "__new__";
    draftName = "";
    draftSynopsis = "";
    draftExample = "";
    errMsg = null;
  }

  function startEdit(f: PsFunction) {
    editingName = f.name;
    draftName = f.name;
    draftSynopsis = f.synopsis;
    draftExample = f.first_example || f.name;
    errMsg = null;
  }

  function cancelEdit() {
    editingName = null;
    errMsg = null;
  }

  async function save() {
    if (!draftName.trim() || !draftSynopsis.trim()) {
      errMsg = "命令名与说明不能为空";
      return;
    }
    busy = true;
    errMsg = null;
    try {
      await api.upsertFunction(
        fileName,
        draftName.trim(),
        draftSynopsis.trim(),
        draftExample.trim() || draftName.trim(),
        null,
        `更新命令 ${draftName.trim()}`
      );
      editingName = null;
      await onChanged();
      showToast(`已保存命令 ${draftName.trim()}`, "success");
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
      await onChanged();
      showToast(`已删除 ${funcName}`, "info");
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
  <div class="flex items-center justify-between mb-2">
    <h3 class="text-sm font-semibold text-slate-300">命令列表</h3>
    <button
      class="px-2 py-1 text-xs bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50"
      onclick={startAdd}
      disabled={busy}>+ 添加命令</button
    >
  </div>

  {#if errMsg}
    <div class="mb-2 p-2 text-xs bg-red-900/40 border border-red-700 text-red-200 rounded">{errMsg}</div>
  {/if}

  {#if editingName}
    <div class="mb-3 p-3 bg-slate-800/60 border border-slate-700 rounded space-y-2">
      <div class="grid grid-cols-3 gap-2">
        <input
          bind:value={draftName}
          placeholder="命令名"
          disabled={editingName !== "__new__"}
          class="px-2 py-1 text-xs bg-slate-950 border border-slate-700 rounded font-mono" />
        <input
          bind:value={draftSynopsis}
          placeholder="说明"
          class="px-2 py-1 text-xs bg-slate-950 border border-slate-700 rounded col-span-2" />
      </div>
      <input
        bind:value={draftExample}
        placeholder="示例（如 gs）"
        class="w-full px-2 py-1 text-xs bg-slate-950 border border-slate-700 rounded font-mono" />
      <div class="flex gap-2">
        <button class="px-2 py-1 text-xs bg-cyan-600 hover:bg-cyan-500 rounded" onclick={save} disabled={busy}>保存</button>
        <button class="px-2 py-1 text-xs bg-slate-700 hover:bg-slate-600 rounded" onclick={cancelEdit}>取消</button>
      </div>
    </div>
  {/if}

  <table class="w-full text-xs">
    <thead class="text-slate-400 border-b border-slate-700">
      <tr>
        <th class="text-left py-1.5 pr-2">命令</th>
        <th class="text-left py-1.5 pr-2">说明</th>
        <th class="text-left py-1.5 pr-2">示例</th>
        <th class="text-right py-1.5">操作</th>
      </tr>
    </thead>
    <tbody>
      {#each functions as f (f.name)}
        <tr class="border-b border-slate-800 align-top">
          <td class="py-1.5 pr-2 font-mono text-cyan-200">{f.name}</td>
          <td class="py-1.5 pr-2 text-slate-300">{f.synopsis || "(无说明)"}</td>
          <td class="py-1.5 pr-2 font-mono text-slate-400">{f.first_example || f.name}</td>
          <td class="py-1.5 text-right whitespace-nowrap">
            <button class="text-cyan-400 hover:text-cyan-200 mr-2" onclick={() => startEdit(f)} disabled={busy}>编辑</button>
            <button class="text-emerald-400 hover:text-emerald-200 mr-2" onclick={() => test(f.name)} disabled={busy}>测试</button>
            <button class="text-amber-400 hover:text-amber-200 mr-2" onclick={() => onAiGenerate(f)} disabled={busy}>AI</button>
            <button class="text-red-400 hover:text-red-300" onclick={() => remove(f.name)} disabled={busy}>删</button>
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
</div>
