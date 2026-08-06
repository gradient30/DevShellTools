<script lang="ts">
  import { toasts, dismissToast, type ToastItem } from "../stores/toast";

  function classes(t: ToastItem): string {
    const base =
      "pointer-events-auto max-w-lg w-full px-4 py-3 rounded-lg border shadow-lg text-sm backdrop-blur-sm animate-[toast-in_0.2s_ease-out]";
    if (t.kind === "success") return `${base} bg-dst-success-bg border-dst-success text-dst-success-fg`;
    if (t.kind === "error") return `${base} bg-dst-danger-bg border-dst-danger-border text-dst-danger-fg`;
    return `${base} bg-dst-elevated border-dst-border text-dst-fg`;
  }
</script>

<div class="fixed top-3 left-1/2 -translate-x-1/2 z-[100] flex flex-col gap-2 w-[min(92vw,32rem)] pointer-events-none">
  {#each $toasts as t (t.id)}
    <div class={classes(t)} role="status">
      <div class="flex items-start justify-between gap-3">
        <pre class="whitespace-pre-wrap font-sans text-sm leading-relaxed flex-1">{t.message}</pre>
        <button class="text-dst-fg-muted hover:text-dst-fg shrink-0" onclick={() => dismissToast(t.id)} aria-label="关闭">×</button>
      </div>
    </div>
  {/each}
</div>

<style>
  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
