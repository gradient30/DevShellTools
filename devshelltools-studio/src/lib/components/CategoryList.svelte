<script lang="ts">
  import type { CategoryInfo } from "../api";

  let {
    categories,
    selectedFileName,
    onSelect
  }: {
    categories: CategoryInfo[];
    selectedFileName: string | null;
    onSelect: (fileName: string) => void;
  } = $props();
</script>

<aside class="w-60 shrink-0 bg-slate-900/60 border-r border-slate-700 overflow-y-auto">
  <div class="p-3 border-b border-slate-700 sticky top-0 bg-slate-900/80 backdrop-blur">
    <h2 class="text-sm font-semibold text-cyan-300">分类</h2>
    <p class="text-xs text-slate-500 mt-0.5">{categories.length} 个</p>
  </div>
  <ul class="py-1">
    {#each categories as c (c.file_name)}
      <li>
        <button
          class="w-full text-left px-3 py-2 text-sm hover:bg-slate-800 transition-colors {selectedFileName ===
          c.file_name
            ? 'bg-cyan-900/40 border-l-2 border-cyan-400 text-cyan-200'
            : 'text-slate-300 border-l-2 border-transparent'}"
          onclick={() => onSelect(c.file_name)}
        >
          <div class="font-medium">{c.category.title}</div>
          <div class="text-xs text-slate-500 mt-0.5">
            <code>{c.category.name}</code> · {c.functions.length} 命令
          </div>
        </button>
      </li>
    {/each}
  </ul>
</aside>