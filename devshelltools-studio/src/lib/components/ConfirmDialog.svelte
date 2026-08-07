<script lang="ts">
  let {
    open = false,
    title = "确认",
    message = "",
    confirmText = "确定",
    cancelText = "取消 (Esc)",
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
      class="bg-dst-surface border border-dst-border rounded-lg w-full max-w-md p-5 shadow-xl"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirm-dialog-title"
      aria-describedby="confirm-dialog-message">
      <h3
        id="confirm-dialog-title"
        class="text-base font-semibold mb-2 {tone === 'danger'
          ? 'text-dst-warning'
          : 'text-dst-fg'}">
        {title}
      </h3>
      <p id="confirm-dialog-message" class="text-sm text-dst-fg-muted leading-relaxed whitespace-pre-wrap">
        {message}
      </p>
      <div class="flex flex-wrap gap-2 pt-4">
        <button
          type="button"
          class="px-3 py-1.5 text-xs rounded transition-colors disabled:opacity-50 {tone === 'danger'
            ? 'bg-dst-warning text-dst-warning-fg hover:opacity-90'
            : 'bg-dst-accent text-dst-accent-fg hover:bg-dst-accent-hover'}"
          onclick={onConfirm}
          disabled={busy}>
          {confirmText}
        </button>
        <button
          type="button"
          class="px-3 py-1.5 text-xs bg-dst-muted hover:bg-dst-muted rounded transition-colors disabled:opacity-50"
          onclick={onCancel}
          disabled={busy}>
          {cancelText}
        </button>
      </div>
    </div>
  </div>
{/if}
