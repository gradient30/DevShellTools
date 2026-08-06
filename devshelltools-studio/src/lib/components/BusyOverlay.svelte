<script lang="ts">
  import { appBusy } from "../stores/busy";
</script>

{#if $appBusy.active}
  <div
    class="fixed inset-0 z-[100] bg-dst-bg/55 backdrop-blur-[1px] flex items-center justify-center"
    role="status"
    aria-live="polite"
    aria-busy="true">
    <div class="mx-4 max-w-sm w-full rounded-xl border border-dst-border bg-dst-surface px-5 py-4 shadow-xl">
      <div class="flex items-center gap-3">
        <span
          class="inline-block h-5 w-5 shrink-0 rounded-full border-2 border-dst-accent border-t-transparent animate-spin"
          aria-hidden="true"></span>
        <div class="min-w-0">
          <p class="text-sm font-medium text-dst-fg">处理中</p>
          <p class="text-xs text-dst-fg-muted mt-0.5 break-words">{$appBusy.message || "请稍候…"}</p>
        </div>
      </div>
    </div>
  </div>
{/if}
