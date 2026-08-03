<script lang="ts">
  import { api, type ChatMessage, type ValidatedCodeBlock } from "../api";

  let {
    onApplyCode,
    onOpenSettings
  }: {
    onApplyCode: (code: string, category: string | null) => Promise<void>;
    onOpenSettings: () => void;
  } = $props();

  let messages = $state<ChatMessage[]>([]);
  let input = $state("");
  let loading = $state(false);
  let errMsg = $state<string | null>(null);
  let reply = $state("");
  let codeBlocks = $state<ValidatedCodeBlock[]>([]);
  let applying = $state(false);

  async function send() {
    if (!input.trim() || loading) return;
    const userMsg: ChatMessage = { role: "user", content: input.trim() };
    messages = [...messages, userMsg];
    input = "";
    loading = true;
    errMsg = null;
    reply = "";
    codeBlocks = [];

    try {
      const result = await api.aiChatWithValidation(messages);
      reply = result.reply;
      codeBlocks = result.code_blocks;
      messages = [...messages, { role: "assistant", content: result.reply }];
    } catch (e) {
      errMsg = String(e);
    } finally {
      loading = false;
    }
  }

  async function apply(block: ValidatedCodeBlock) {
    if (!block.syntax_ok || !block.safety_ok) return;
    applying = true;
    try {
      await onApplyCode(block.code, block.category);
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

<aside class="w-96 shrink-0 bg-slate-900/60 border-l border-slate-700 flex flex-col">
  <div class="p-3 border-b border-slate-700 flex items-center justify-between">
    <h3 class="text-sm font-semibold text-cyan-300">AI 助手</h3>
    <button class="text-xs text-slate-400 hover:text-slate-200" onclick={onOpenSettings}>设置</button>
  </div>

  {#if errMsg}
    <div class="px-3 py-2 bg-red-900/40 border-b border-red-700 text-red-200 text-xs">{errMsg}</div>
  {/if}

  <div class="flex-1 overflow-y-auto p-3 space-y-3">
    {#if messages.length === 0 && !loading}
      <div class="text-xs text-slate-500 text-center py-8">
        描述你想要的命令，AI 会生成 PowerShell 代码。<br />
        例如："生成一个查看 docker 容器日志的命令"
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
      <div class="text-xs text-slate-400 flex items-center gap-2">
        <span class="animate-pulse">●</span> 生成中…
      </div>
    {/if}

    {#if codeBlocks.length > 0}
      <div class="space-y-2 pt-2 border-t border-slate-700">
        <div class="text-xs text-slate-400 font-semibold">提议的代码（{codeBlocks.length}）</div>
        {#each codeBlocks as block, i}
          <div class="bg-slate-800/60 border border-slate-700 rounded p-2">
            <div class="flex items-center gap-2 mb-1">
              <span class="text-xs font-mono text-cyan-200">#{i + 1}</span>
              {#if block.functions.length > 0}
                <span class="text-xs text-slate-300">函数：{block.functions.join(", ")}</span>
              {/if}
              {#if block.category}
                <span class="text-xs text-amber-300">分类：{block.category}</span>
              {/if}
            </div>

            <div class="flex gap-3 text-xs mb-1">
              {#if block.syntax_ok}
                <span class="text-green-400">✓ 语法</span>
              {:else}
                <span class="text-red-400">✗ 语法</span>
              {/if}
              {#if block.safety_ok}
                <span class="text-green-400">✓ 安全</span>
              {:else}
                <span class="text-red-400">✗ 安全</span>
              {/if}
            </div>

            {#if !block.syntax_ok}
              <div class="text-xs text-red-300 mb-1">{block.syntax_err}</div>
            {/if}
            {#if !block.safety_ok}
              <div class="text-xs text-red-300 mb-1">拦截：{block.safety_violations.join("；")}</div>
            {/if}

            <pre class="text-xs font-mono bg-slate-950 border border-slate-800 rounded p-2 overflow-x-auto max-h-32 text-slate-300">{block.code}</pre>

            {#if canApply(block)}
              <button
                class="mt-2 px-3 py-1 text-xs bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50"
                onclick={() => apply(block)}
                disabled={applying}>
                {applying ? "应用中…" : "应用到工作区"}
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
        disabled={loading} />
      <button
        class="px-3 py-1.5 text-sm bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50"
        onclick={send}
        disabled={loading || !input.trim()}>
        发送
      </button>
    </div>
  </div>
</aside>