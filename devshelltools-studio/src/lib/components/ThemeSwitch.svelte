<script lang="ts">
  import { THEME_OPTIONS, getTheme, setTheme, type ThemeId } from "../stores/theme.svelte";

  let theme = $state<ThemeId>(getTheme());

  function pick(id: ThemeId) {
    setTheme(id);
    theme = id;
  }
</script>

<div class="dst-theme-grid" role="group" aria-label="主题切换">
  {#each THEME_OPTIONS as opt (opt.id)}
    <button
      type="button"
      aria-pressed={theme === opt.id}
      title="切换到{opt.label}主题"
      onclick={() => pick(opt.id)}
    >
      {opt.label}
    </button>
  {/each}
</div>
