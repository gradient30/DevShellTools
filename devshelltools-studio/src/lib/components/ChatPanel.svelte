<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AiProfile, type CategoryInfo, type ChatMessage, type ValidatedCodeBlock } from "../api";

  let {
    categories,
    initialPrompt,
    autoSendToken = 0,
    onApplyCode,
    onOpenSettings
  }: {
    categories: CategoryInfo[];
    initialPrompt: string;
    autoSendToken?: number;
    onApplyCode: (code: string, fileName: string) => Promise<void>;
    onOpenSettings: () => void;
  } = $props();

  let profiles = $state<AiProfile[]>([]);
  let profileId = $state("");
  let messages = $state<ChatMessage[]>([]);
  let input = $state("");
  let loading = $state(false);
  let errMsg = $state<string | null>(null);
  let applying = $state(false);
  let replyCodeBlocks = $state<Record<number, ValidatedCodeBlock[]>>({});
  let targetFiles = $state<Record<string, string>>({});
  let expandedBlocks = $state<Set<string>>(new Set());
  let lastAutoToken = $state(0);
  let profilesReady = $state(false);

  onMount(async () => {
    try {
      profiles = await api.listAiProfiles();
      profileId = profiles.find((p) => p.key_configured)?.id ?? profiles[0]?.id ?? "";
    } catch (e) {
      errMsg = String(e);
    } finally {
      profilesReady = true;
    }
    if (initialPrompt && !autoSendToken) input = initialPrompt;
  });

  $effect(() => {
    if (!profilesReady || !profileId) return;
    if (autoSendToken > 0 && autoSendToken !== lastAutoToken && initialPrompt.trim()) {
      lastAutoToken = autoSendToken;
      messages = [];
      replyCodeBlocks = {};
      targetFiles = {};
      expandedBlocks = new Set();
      void sendPrompt(initialPrompt.trim());
      return;
    }
    if (initialPrompt && messages.length === 0 && !loading) {
      input = initialPrompt;
    }
  });

  async function sendPrompt(text: string) {
    if (!text || loading || !profileId) return;
    const userMsg: ChatMessage = { role: "user", content: text };
    messages = [...messages, userMsg];
    input = "";
    loading = true;
    errMsg = null;
    try {
      const result = await api.aiChatWithValidation(messages, profileId);
      const assistantIdx = messages.length;
      messages = [...messages, { role: "assistant", content: result.reply }];
      if (result.code_blocks.length > 0) {
        replyCodeBlocks[assistantIdx] = result.code_blocks;
        result.code_blocks.forEach((block, bi) => {
          if (block.category) {
            const match = categories.find((c) => c.category.name === block.category);
            targetFiles[`${assistantIdx}-${bi}`] = match?.file_name ?? categories[0]?.file_name ?? "";
          } else {
            targetFiles[`${assistantIdx}-${bi}`] = categories[0]?.file_name ?? "";
          }
        });
      }
    } catch (e) {
      errMsg = String(e);
    } finally {
      loading = false;
    }
  }

  async function send() {
    await sendPrompt(input.trim());
  }

  async function apply(block: ValidatedCodeBlock, key: string) {
    if (!block.syntax_ok || !block.safety_ok) return;
    const fileName = targetFiles[key];
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

  function toggleBlock(key: string) {
    if (expandedBlocks.has(key)) expandedBlocks.delete(key);
    else expandedBlocks.add(key);
    expandedBlocks = new Set(expandedBlocks);
  }

  function canApply(block: ValidatedCodeBlock): boolean {
    return block.syntax_ok && block.safety_ok && block.functions.length > 0;
  }

  function rewindTo(index: number) {
    messages = messages.slice(0, index);
    Object.keys(replyCodeBlocks).forEach((k) => {
      if (parseInt(k) >= index) delete replyCodeBlocks[parseInt(k)];
    });
    replyCodeBlocks = { ...replyCodeBlocks };
  }

  function editUserMessage(index: number, content: string) {
    rewindTo(index);
    input = content;
  }

  function textWithoutCode(text: string): string {
    return text.replace(/```[\s\S]*?```/g, "").trim();
  }
</script>

<div class="h-full flex flex-col">
  <div class="px-4 py-2.5 border-b border-slate-700 flex items-center justify-between gap-2">
    <div class="flex items-center gap-2 min-w-0">
      <span class="text-lg">🤖</span>
      <h3 class="text-sm font-semibold text-cyan-300 shrink-0">AI 助手</h3>
      <select bind:value={profileId} class="text-xs bg-slate-800 border border-slate-700 rounded px-2 py-1 text-slate-200 max-w-[180px]">
        {#each profiles as p (p.id)}
          <option value={p.id}>{p.name} · {p.model}</option>
        {/each}
      </select>
    </div>
    <button class="text-xs text-slate-400 hover:text-slate-200 shrink-0" onclick={onOpenSettings}>配置</button>
  </div>

  {#if errMsg}
    <div class="px-4 py-2 bg-red-900/40 border-b border-red-700 text-red-200 text-xs flex justify-between">
      <span>{errMsg}</span>
      <button class="text-red-300" onclick={() => (errMsg = null)}>×</button>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto px-4 py-4 space-y-4">
    {#if messages.length === 0 && !loading}
      <div class="flex flex-col items-center justify-center h-full text-center gap-3">
        <div class="text-4xl">🤖</div>
        <p class="text-sm text-slate-400">可直接提问，或从命令列表点「AI审阅」</p>
        <p class="text-xs text-slate-600">审阅流程：检查问题 → 优化建议 → 可新增命令建议；校验通过后可插入分类</p>
      </div>
    {/if}

    {#each messages as m, i (i)}
      {#if m.role === "user"}
        <div class="flex justify-end gap-2 group">
          <div class="flex flex-col items-end gap-1 max-w-[80%]">
            <div class="bg-cyan-700/40 rounded-lg rounded-tr-sm px-3 py-2 text-sm text-cyan-50 whitespace-pre-wrap break-words">
              {m.content}
            </div>
            <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
              <button class="text-xs text-slate-500 hover:text-cyan-300" onclick={() => editUserMessage(i, m.content)}>编辑</button>
              <button class="text-xs text-slate-500 hover:text-red-300" onclick={() => rewindTo(i)}>回退到此</button>
            </div>
          </div>
          <div class="w-8 h-8 rounded-full bg-cyan-800/60 border border-cyan-700 flex items-center justify-center text-sm shrink-0">👤</div>
        </div>
      {:else}
        <div class="flex justify-start gap-2">
          <div class="w-8 h-8 rounded-full bg-emerald-800/60 border border-emerald-700 flex items-center justify-center text-sm shrink-0">🤖</div>
          <div class="flex flex-col gap-2 max-w-[85%] min-w-0">
            {#if textWithoutCode(m.content)}
              <div class="bg-slate-800/60 rounded-lg rounded-tl-sm px-3 py-2 text-sm text-slate-200 whitespace-pre-wrap break-words">
                {textWithoutCode(m.content)}
              </div>
            {/if}
            {#if replyCodeBlocks[i]}
              {#each replyCodeBlocks[i] as block, bi}
                {@const key = `${i}-${bi}`}
                <div class="border border-slate-700 rounded-lg overflow-hidden">
                  <div
                    class="flex items-center justify-between px-3 py-1.5 bg-slate-800/80 cursor-pointer hover:bg-slate-700/60"
                    role="button"
                    tabindex="0"
                    onclick={() => toggleBlock(key)}
                    onkeydown={(e) => e.key === "Enter" && toggleBlock(key)}>
                    <div class="flex items-center gap-2 text-xs">
                      <span class="text-slate-400 font-mono">{expandedBlocks.has(key) ? "▾" : "▸"}</span>
                      <span class="text-cyan-300 font-mono">{block.functions.join(", ") || "代码块"}</span>
                      <span class="text-slate-500">{block.syntax_ok ? "语法✓" : "语法✗"} · {block.safety_ok ? "安全✓" : "安全✗"}</span>
                    </div>
                  </div>
                  {#if expandedBlocks.has(key)}
                    <div class="px-3 py-2 bg-slate-950/50 space-y-2">
                      <pre class="text-xs text-slate-300 overflow-x-auto whitespace-pre-wrap font-mono">{block.code}</pre>
                      {#if !block.syntax_ok || !block.safety_ok}
                        <p class="text-xs text-amber-300">
                          {[block.syntax_err, ...block.safety_violations].filter(Boolean).join("；") || "未通过校验"}
                        </p>
                      {/if}
                      <div class="flex items-center gap-2">
                        <select bind:value={targetFiles[key]} class="text-xs bg-slate-800 border border-slate-700 rounded px-2 py-1">
                          {#each categories as c}
                            <option value={c.file_name}>{c.category.title}</option>
                          {/each}
                        </select>
                        <button
                          class="px-2 py-1 text-xs bg-emerald-700 hover:bg-emerald-600 rounded disabled:opacity-40"
                          disabled={!canApply(block) || applying}
                          onclick={() => apply(block, key)}>
                          插入到分类
                        </button>
                      </div>
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    {/each}

    {#if loading}
      <div class="flex justify-start gap-2">
        <div class="w-8 h-8 rounded-full bg-emerald-800/60 border border-emerald-700 flex items-center justify-center text-sm shrink-0">🤖</div>
        <div class="flex items-center gap-2 bg-slate-800/60 rounded-lg rounded-tl-sm px-3 py-2">
          <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 0ms"></span>
          <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 150ms"></span>
          <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 300ms"></span>
          <span class="text-xs text-slate-400 ml-1">正在审阅命令…</span>
        </div>
      </div>
    {/if}
  </div>

  <div class="p-3 border-t border-slate-700">
    <div class="flex gap-2">
      <input
        bind:value={input}
        onkeydown={(e) => e.key === "Enter" && !e.shiftKey && (e.preventDefault(), send())}
        placeholder="继续追问，或描述新命令…"
        class="flex-1 px-3 py-2 text-sm bg-slate-800 border border-slate-700 rounded-lg text-slate-200 focus:border-cyan-600 focus:outline-none"
        disabled={loading || !profileId} />
      <button
        class="px-4 py-2 text-sm bg-cyan-600 hover:bg-cyan-500 rounded-lg disabled:opacity-50 transition-colors"
        onclick={send}
        disabled={loading || !input.trim() || !profileId}>
        发送
      </button>
    </div>
  </div>
</div>
