<script lang="ts">
  import type { CategoryInfo } from "../api";

  let {
    categories,
    selectedFileName,
    loading = false,
    onSelect
  }: {
    categories: CategoryInfo[];
    selectedFileName: string | null;
    loading?: boolean;
    onSelect: (fileName: string) => void;
  } = $props();
</script>

<aside class="w-60 shrink-0 bg-dst-surface border-r border-dst-border overflow-y-auto">
  <div class="p-3 border-b border-dst-border sticky top-0 bg-dst-surface backdrop-blur">
    <h2 class="text-sm font-semibold text-dst-accent">分类</h2>
    <p class="text-xs text-dst-fg-muted mt-0.5">{loading ? "加载中…" : `${categories.length} 个`}</p>
  </div>
  {#if loading}
    <div class="p-3 space-y-2">
      {#each [1, 2, 3, 4, 5] as _}
        <div class="h-12 bg-dst-elevated/50 rounded animate-pulse"></div>
      {/each}
    </div>
  {:else}
  <ul class="py-1">
    {#each categories as c (c.file_name)}
      <li>
        <button
          class="w-full text-left px-3 py-2 text-sm hover:bg-dst-elevated transition-colors {selectedFileName ===
          c.file_name
            ? 'bg-dst-elevated border-l-2 border-dst-accent text-dst-accent'
            : 'text-dst-fg border-l-2 border-transparent'}"
          onclick={() => onSelect(c.file_name)}
        >
          <div class="font-medium">{c.category.title}</div>
          <div class="text-xs text-dst-fg-muted mt-0.5">
            <code>{c.category.name}</code> · {c.functions.filter((f) => /^[a-z]/.test(f.name)).length} 命令
          </div>
        </button>
      </li>
    {/each}
  </ul>
  {/if}
</aside>