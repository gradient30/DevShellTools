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
  let applying = $state(false);
  // 每条 AI 回复关联的代码块
  let replyCodeBlocks = $state<Record<number, ValidatedCodeBlock[]>>({});
  let targetFiles = $state<Record<string, string>>({});
  let expandedBlocks = $state<Set<string>>(new Set());

  onMount(async () => {
    try {
      profiles = await api.listAiProfiles();
      profileId = profiles.find((p) => p.key_configured)?.id ?? profiles[0]?.id ?? "";
    } catch (e) {
      errMsg = String(e);
    }
    if (initialPrompt) input = initialPrompt;
  });

  $effect(() => {
    if (initialPrompt && !messages.length) input = initialPrompt;
  });

  async function send() {
    if (!input.trim() || loading || !profileId) return;
    const userMsg: ChatMessage = { role: "user", content: input.trim() };
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
        // 自动推荐分类：按 AI 回复中的 category 字段匹配
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

  async function apply(block: ValidatedCodeBlock, key: string) {
    if (!block.syntax_ok || !block.safety_ok) return;
    const fileName = targetFiles[key];
    if (!fileName) { errMsg = "请选择目标分类"; return; }
    applying = true;
    try {
      await onApplyCode(block.code, fileName);
      expandedBlocks.delete(key);
      expandedBlocks = new Set(expandedBlocks);
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

  // rewind：删除从指定位置开始的所有后续消息
  function rewindTo(index: number) {
    messages = messages.slice(0, index);
    // 清理关联的代码块
    Object.keys(replyCodeBlocks).forEach((k) => {
      if (parseInt(k) >= index) delete replyCodeBlocks[parseInt(k)];
    });
    replyCodeBlocks = { ...replyCodeBlocks };
  }

  // 编辑用户消息：恢复到该消息并放入输入框
  function editUserMessage(index: number, content: string) {
    rewindTo(index);
    input = content;
  }

  // 提取代码块外的文字（去掉 ```powershell 块）
  function textWithoutCode(text: string): string {
    return text.replace(/```[\s\S]*?```/g, "").trim();
  }

  // 从回复中提取代码块（简单分割）
  function extractCodeBlocks(text: string): string[] {
    const blocks: string[] = [];
    const regex = /```(?:powershell|ps1)?\n([\s\S]*?)```/g;
    let match;
    while ((match = regex.exec(text)) !== null) blocks.push(match[1]);
    return blocks;
  }
</script>

<div class="h-full flex flex-col">
  <!-- 顶部栏 -->
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

  <!-- 对话区 -->
  <div class="flex-1 overflow-y-auto px-4 py-4 space-y-4">
    {#if messages.length === 0 && !loading}
      <div class="flex flex-col items-center justify-center h-full text-center gap-3">
        <div class="text-4xl">🤖</div>
        <p class="text-sm text-slate-400">描述你想要的 PowerShell 命令，AI 会生成代码</p>
        <p class="text-xs text-slate-600">校验通过后可插入到指定分类</p>
      </div>
    {/if}

    {#each messages as m, i (i)}
      {#if m.role === "user"}
        <!-- 用户消息：右侧 -->
        <div class="flex justify-end gap-2 group">
          <div class="flex flex-col items-end gap-1 max-w-[80%]">
            <div class="bg-cyan-700/40 rounded-lg rounded-tr-sm px-3 py-2 text-sm text-cyan-50 whitespace-pre-wrap break-words">
              {m.content}
            </div>
            <!-- 编辑/rewind 按钮（hover 显示） -->
            <div class="flex gap-2 opacity-0 group-hover:opacity-100 transition-opacity">
              <button class="text-xs text-slate-500 hover:text-cyan-300" onclick={() => editUserMessage(i, m.content)}>编辑</button>
              <button class="text-xs text-slate-500 hover:text-red-300" onclick={() => rewindTo(i)}>回退到此</button>
            </div>
          </div>
          <div class="w-8 h-8 rounded-full bg-cyan-800/60 border border-cyan-700 flex items-center justify-center text-sm shrink-0">👤</div>
        </div>
      {:else}
        <!-- AI 消息：左侧 -->
        <div class="flex justify-start gap-2">
          <div class="w-8 h-8 rounded-full bg-emerald-800/60 border border-emerald-700 flex items-center justify-center text-sm shrink-0">🤖</div>
          <div class="flex flex-col gap-2 max-w-[85%]">
            <!-- 文字部分（不含代码块） -->
            {#if textWithoutCode(m.content)}
              <div class="bg-slate-800/60 rounded-lg rounded-tl-sm px-3 py-2 text-sm text-slate-200 whitespace-pre-wrap break-words">
                {textWithoutCode(m.content)}
              </div>
            {/if}
            <!-- 代码块（折叠/展开） -->
            {#if replyCodeBlocks[i]?.length > 0}
              {#each replyCodeBlocks[i] as block, bi (bi)}
                {@const key = `${i}-${bi}`}
                <div class="border border-slate-700 rounded-lg overflow-hidden">
                  <!-- 代码块头部 -->
                  <div class="flex items-center justify-between px-3 py-1.5 bg-slate-800/80 cursor-pointer hover:bg-slate-700/60" onclick={() => toggleBlock(key)}>
                    <div class="flex items-center gap-2 text-xs">
                      <span class="text-slate-400 font-mono">{expandedBlocks.has(key) ? "▾" : "▸"}</span>
                      <span class="text-cyan-300 font-mono">{block.functions.join(", ") || "代码块"}</span>
                      {#if block.category}<span class="text-amber-400">→ {block.category}</span>{/if}
                      <span class={block.syntax_ok ? "text-green-400" : "text-red-400"}>{block.syntax_ok ? "✓语法" : "✗语法"}</span>
                      <span class={block.safety_ok ? "text-green-400" : "text-red-400"}>{block.safety_ok ? "✓安全" : "✗安全"}</span>
                    </div>
                    <button class="text-xs text-slate-500 hover:text-cyan-300" onclick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(block.code); }}>复制</button>
                  </div>
                  <!-- 展开内容 -->
                  {#if expandedBlocks.has(key)}
                    <div class="bg-slate-950 p-2">
                      <pre class="text-xs font-mono text-slate-300 overflow-x-auto whitespace-pre max-h-60">{block.code}</pre>
                      {#if canApply(block)}
                        <div class="flex items-center gap-2 mt-2">
                          <span class="text-xs text-slate-500">插入到：</span>
                          <select bind:value={targetFiles[key]} class="text-xs bg-slate-800 border border-slate-700 rounded px-2 py-1 text-slate-200">
                            {#each categories as c}
                              <option value={c.file_name}>{c.category.title}</option>
                            {/each}
                          </select>
                          <button class="px-3 py-1 text-xs bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50" onclick={() => apply(block, key)} disabled={applying}>
                            {applying ? "插入中…" : "插入"}
                          </button>
                        </div>
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>
      {/if}
    {/each}

    <!-- 思考中状态 -->
    {#if loading}
      <div class="flex justify-start gap-2">
        <div class="w-8 h-8 rounded-full bg-emerald-800/60 border border-emerald-700 flex items-center justify-center text-sm shrink-0">🤖</div>
        <div class="flex items-center gap-2 bg-slate-800/60 rounded-lg rounded-tl-sm px-3 py-2">
          <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 0ms"></span>
          <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 150ms"></span>
          <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 300ms"></span>
          <span class="text-xs text-slate-400 ml-1">思考中…</span>
        </div>
      </div>
    {/if}
  </div>

  <!-- 输入区 -->
  <div class="p-3 border-t border-slate-700">
    <div class="flex gap-2">
      <input
        bind:value={input}
        onkeydown={(e) => e.key === "Enter" && !e.shiftKey && (e.preventDefault(), send())}
        placeholder="描述你想要的命令…"
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