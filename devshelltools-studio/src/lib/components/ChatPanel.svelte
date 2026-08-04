<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AiProfile, type CategoryInfo, type ChatMessage, type ValidatedCodeBlock } from "../api";

  let {
    categories,
    initialPrompt,
    onApplyCode,
    onOpenSettings
  }: {
    categories: CategoryInfo[];
    initialPrompt: string;
    onApplyCode: (code: string, fileName: string) => Promise<void>;
    onOpenSettings: () => void;
  } = $props();

  let profiles = $state<AiProfile[]>([]);
  let profileId = $state("");
  let messages = $state<ChatMessage[]>([]);
  let input = $state("");
  let loading = $state(false);
  let errMsg = $state<string | null>(null);
  let reply = $state("");
  let codeBlocks = $state<ValidatedCodeBlock[]>([]);
  let applying = $state(false);
  let targetFiles = $state<Record<number, string>>({});

  onMount(async () => {
    try {
      profiles = await api.listAiProfiles();
      profileId = profiles.find((p) => p.key_configured)?.id ?? profiles[0]?.id ?? "";
    } catch (e) {
      errMsg = String(e);
    }
    if (initialPrompt) {
      input = initialPrompt;
    }
  });

  $effect(() => {
    if (initialPrompt && !messages.length) {
      input = initialPrompt;
    }
  });

  async function send() {
    if (!input.trim() || loading || !profileId) return;
    const userMsg: ChatMessage = { role: "user", content: input.trim() };
    messages = [...messages, userMsg];
    input = "";
    loading = true;
    errMsg = null;
    reply = "";
    codeBlocks = [];
    targetFiles = {};

    try {
      const result = await api.aiChatWithValidation(messages, profileId);
      reply = result.reply;
      codeBlocks = result.code_blocks;
      const defaultFile = categories[0]?.file_name ?? "";
      targetFiles = Object.fromEntries(codeBlocks.map((_, i) => [i, defaultFile]));
      messages = [...messages, { role: "assistant", content: result.reply }];
    } catch (e) {
      errMsg = String(e);
    } finally {
      loading = false;
    }
  }

  async function apply(block: ValidatedCodeBlock, index: number) {
    if (!block.syntax_ok || !block.safety_ok) return;
    const fileName = targetFiles[index];
    if (!fileName) {
      errMsg = "请选择目标分类";
      return;
    }
    applying = true;
    try {
      await onApplyCode(block.code, fileName);
    } catch (e) {
      errMsg = String(e);
    } finally {
      applying = false;
    }
  }

  function canApply(block: ValidatedCodeBlock): boolean {
    return block.syntax_ok && block.safety_ok && block.functions.length > 0;
  }
</script>

<div class="h-full flex flex-col">
  <div class="p-3 border-b border-slate-700 flex items-center justify-between gap-2">
    <div class="flex items-center gap-2 min-w-0">
      <h3 class="text-sm font-semibold text-cyan-300 shrink-0">AI 助手</h3>
      <select
        bind:value={profileId}
        class="text-xs bg-slate-800 border border-slate-700 rounded px-2 py-1 text-slate-200 max-w-[200px]">
        {#each profiles as p (p.id)}
          <option value={p.id}>{p.name} ({p.model})</option>
        {/each}
      </select>
    </div>
    <button class="text-xs text-slate-400 hover:text-slate-200 shrink-0" onclick={onOpenSettings}>管理配置</button>
  </div>

  {#if errMsg}
    <div class="px-3 py-2 bg-red-900/40 border-b border-red-700 text-red-200 text-xs">{errMsg}</div>
  {/if}

  <div class="flex-1 overflow-y-auto p-3 space-y-3">
    {#if messages.length === 0 && !loading}
      <div class="text-xs text-slate-500 text-center py-8">
        描述你想要的命令，AI 会生成 PowerShell 代码。<br />
        校验通过后可插入到指定分类。
      </div>
    {/if}

    {#each messages as m}
      <div class="text-xs {m.role === 'user' ? 'text-cyan-200' : 'text-slate-300'}">
        <span class="font-semibold {m.role === 'user' ? 'text-cyan-400' : 'text-emerald-400'}">
          {m.role === "user" ? "你" : "AI"}
        </span>
        <div class="mt-1 whitespace-pre-wrap break-words max-h-40 overflow-y-auto">{m.content}</div>
      </div>
    {/each}

    {#if loading}
      <div class="text-xs text-slate-400 flex items-center gap-2"><span class="animate-pulse">●</span> 生成中…</div>
    {/if}

    {#if codeBlocks.length > 0}
      <div class="space-y-2 pt-2 border-t border-slate-700">
        <div class="text-xs text-slate-400 font-semibold">提议的代码（{codeBlocks.length}）</div>
        {#each codeBlocks as block, i}
          <div class="bg-slate-800/60 border border-slate-700 rounded p-2">
            <div class="flex flex-wrap items-center gap-2 mb-1 text-xs">
              <span class="font-mono text-cyan-200">#{i + 1}</span>
              {#if block.functions.length > 0}
                <span class="text-slate-300">函数：{block.functions.join(", ")}</span>
              {/if}
            </div>
            <div class="flex gap-3 text-xs mb-2">
              <span class={block.syntax_ok ? "text-green-400" : "text-red-400"}>{block.syntax_ok ? "✓ 语法" : "✗ 语法"}</span>
              <span class={block.safety_ok ? "text-green-400" : "text-red-400"}>{block.safety_ok ? "✓ 安全" : "✗ 安全"}</span>
            </div>
            <label class="text-xs text-slate-400 block mb-1">插入到分类</label>
            <select
              bind:value={targetFiles[i]}
              class="w-full mb-2 text-xs bg-slate-950 border border-slate-700 rounded px-2 py-1">
              {#each categories as c}
                <option value={c.file_name}>{c.category.title} ({c.file_name})</option>
              {/each}
            </select>
            <pre class="text-xs font-mono bg-slate-950 border border-slate-800 rounded p-2 overflow-x-auto max-h-32 text-slate-300">{block.code}</pre>
            {#if canApply(block)}
              <button
                class="mt-2 px-3 py-1 text-xs bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50"
                onclick={() => apply(block, i)}
                disabled={applying}>
                {applying ? "插入中…" : "插入到分类"}
              </button>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div class="p-3 border-t border-slate-700">
    <div class="flex gap-2">
      <input
        bind:value={input}
        onkeydown={(e) => e.key === "Enter" && send()}
        placeholder="描述需求…"
        class="flex-1 px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded text-slate-200"
        disabled={loading || !profileId} />
      <button
        class="px-3 py-1.5 text-sm bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50"
        onclick={send}
        disabled={loading || !input.trim() || !profileId}>发送</button>
    </div>
  </div>
</div>
