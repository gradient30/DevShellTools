<script lang="ts">
  let {
    open = false,
    title = "确认",
    message = "",
    confirmText = "确定",
    cancelText = "取消",
    tone = "default" as "default" | "danger",
    busy = false,
    onConfirm,
    onCancel
  }: {
    open?: boolean;
    title?: string;
    message?: string;
    confirmText?: string;
    cancelText?: string;
    tone?: "default" | "danger";
    busy?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();

  function onBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget && !busy) onCancel();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && open && !busy) {
      e.preventDefault();
      e.stopPropagation();
      onCancel();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <div
    class="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4"
    onclick={onBackdropClick}
    role="presentation">
    <div
      class="bg-dst-surface border rounded-lg w-full max-w-md shadow-xl overflow-hidden {tone === 'danger'
        ? 'border-dst-danger-border'
        : 'border-dst-border'}"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirm-dialog-title"
      aria-describedby="confirm-dialog-message">
      <div class="px-5 pt-5 pb-4">
        <div class="flex items-start gap-3">
          {#if tone === "danger"}
            <span
              class="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-dst-danger-bg border border-dst-danger-border text-lg"
              aria-hidden="true">⚠</span>
          {/if}
          <div class="min-w-0 flex-1">
            <h3
              id="confirm-dialog-title"
              class="text-base font-semibold {tone === 'danger'
                ? 'text-dst-danger-fg'
                : 'text-dst-fg'}">
              {title}
            </h3>
            <p
              id="confirm-dialog-message"
              class="text-sm leading-relaxed mt-2 whitespace-pre-wrap {tone === 'danger'
                ? 'text-dst-fg-muted'
                : 'text-dst-fg-muted'}">
              {message}
            </p>
          </div>
        </div>
      </div>

      <div
        class="px-5 py-4 border-t flex justify-center gap-3 {tone === 'danger'
          ? 'border-dst-danger-border/50 bg-dst-danger-bg/25'
          : 'border-dst-border bg-dst-elevated/40'}">
        <button
          type="button"
          class="min-w-[6.5rem] px-4 py-2 text-sm rounded-lg border transition-colors disabled:opacity-50 {tone === 'danger'
            ? 'border-dst-border bg-dst-surface text-dst-fg hover:bg-dst-elevated'
            : 'border-dst-border bg-dst-surface text-dst-fg hover:bg-dst-elevated'}"
          onclick={onCancel}
          disabled={busy}>
          {cancelText}
        </button>
        <button
          type="button"
          class="min-w-[6.5rem] px-4 py-2 text-sm rounded-lg transition-colors disabled:opacity-50 {tone === 'danger'
            ? 'bg-dst-btn-danger text-dst-btn-danger-fg hover:opacity-90'
            : 'bg-dst-accent text-dst-accent-fg hover:bg-dst-accent-hover'}"
          onclick={onConfirm}
          disabled={busy}>
          {busy ? "处理中…" : confirmText}
        </button>
      </div>
    </div>
  </div>
{/if}
