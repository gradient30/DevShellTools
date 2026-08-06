<script lang="ts">
  import { markdownToHtml } from "../markdown";

  let { source = "" }: { source: string } = $props();

  let html = $derived(markdownToHtml(source));
</script>

{#if html}
  <div class="dst-md break-words">{@html html}</div>
{/if}
