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
  /** 编辑/回退进入发送框时的会话快照，用于「取消」恢复 */
  let draftSnapshot = $state<{
    messages: ChatMessage[];
    replyCodeBlocks: Record<number, ValidatedCodeBlock[]>;
    targetFiles: Record<string, string>;
    expandedBlocks: string[];
    input: string;
  } | null>(null);
  let draftMode = $state<"edit" | "rewind" | null>(null);

  /** 加载配置列表，并始终对齐设置页的默认 Profile */
  async function loadProfiles(syncDefault = true) {
    const meta = await api.getAiProfilesMeta();
    profiles = meta.profiles;
    const defaultId = meta.default_profile_id;
    const defaultExists = !!defaultId && profiles.some((p) => p.id === defaultId);
    if (syncDefault && defaultExists) {
      profileId = defaultId!;
      return;
    }
    // 默认无效时：保留当前选择（若仍存在），否则回退到已配 Key / 首项
    if (profileId && profiles.some((p) => p.id === profileId)) return;
    profileId =
      (defaultExists ? defaultId! : null) ??
      profiles.find((p) => p.key_configured)?.id ??
      profiles[0]?.id ??
      "";
  }

  onMount(() => {
    void (async () => {
      try {
        await loadProfiles(true);
      } catch (e) {
        errMsg = String(e);
      } finally {
        profilesReady = true;
      }
      if (initialPrompt && !autoSendToken) input = initialPrompt;
    })();

    const onConfigChanged = () => {
      void loadProfiles(true).catch((e) => {
        errMsg = String(e);
      });
    };
    window.addEventListener("ai-config-changed", onConfigChanged);
    return () => window.removeEventListener("ai-config-changed", onConfigChanged);
  });

  $effect(() => {
    if (!profilesReady || !profileId) return;
    if (autoSendToken > 0 && autoSendToken !== lastAutoToken && initialPrompt.trim()) {
      lastAutoToken = autoSendToken;
      messages = [];
      replyCodeBlocks = {};
      targetFiles = {};
      expandedBlocks = new Set();
      clearDraftState();
      void sendPrompt(initialPrompt.trim());
      return;
    }
    // 编辑/回退草稿中禁止用 initialPrompt 覆盖发送框（否则点「编辑」像没反应）
    if (draftMode) return;
    if (initialPrompt && messages.length === 0 && !loading && !input.trim()) {
      input = initialPrompt;
    }
  });

  async function sendPrompt(text: string) {
    if (!text || loading || !profileId) return;
    const userMsg: ChatMessage = { role: "user", content: text };
    messages = [...messages, userMsg];
    input = "";
    clearDraftState();
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
        // 同命令迭代：收起旧稿，仅展开有变更的最新脚本框
        focusLatestBlocks(assistantIdx, result.code_blocks);
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

  /** 用函数名标识「同一脚本」；无函数名则各块独立（不同匿名脚本都展示） */
  function scriptIdentity(block: ValidatedCodeBlock): string | null {
    if (block.functions.length === 0) return null;
    return [...block.functions].map((f) => f.toLowerCase()).sort().join(",");
  }

  function normalizeCode(code: string): string {
    return code.replace(/\r\n/g, "\n").trim();
  }

  /** 各脚本身份 → 对话中最新代码块 key（msgIdx-blockIdx） */
  let latestKeyByScript = $derived.by(() => {
    const map = new Map<string, string>();
    const idxs = Object.keys(replyCodeBlocks)
      .map((k) => parseInt(k, 10))
      .filter((n) => !Number.isNaN(n))
      .sort((a, b) => a - b);
    for (const i of idxs) {
      const blocks = replyCodeBlocks[i] ?? [];
      blocks.forEach((block, bi) => {
        const id = scriptIdentity(block);
        if (id) map.set(id, `${i}-${bi}`);
      });
    }
    return map;
  });

  function isLatestScriptBlock(block: ValidatedCodeBlock, key: string): boolean {
    const id = scriptIdentity(block);
    if (!id) return true;
    return latestKeyByScript.get(id) === key;
  }

  function findEarlierBlock(
    identity: string,
    beforeMsgIdx: number,
    beforeBlockIdx = Infinity
  ): ValidatedCodeBlock | null {
    for (let i = beforeMsgIdx; i >= 0; i--) {
      const blocks = replyCodeBlocks[i];
      if (!blocks) continue;
      const maxBi = i === beforeMsgIdx ? Math.min(beforeBlockIdx, blocks.length) - 1 : blocks.length - 1;
      for (let bi = maxBi; bi >= 0; bi--) {
        if (scriptIdentity(blocks[bi]) === identity) return blocks[bi];
      }
    }
    return null;
  }

  function isUnchangedFromPrevious(
    block: ValidatedCodeBlock,
    msgIdx: number,
    blockIdx: number
  ): boolean {
    const id = scriptIdentity(block);
    if (!id) return false;
    const earlier = findEarlierBlock(id, msgIdx, blockIdx);
    if (!earlier) return false;
    return normalizeCode(earlier.code) === normalizeCode(block.code);
  }

  function focusLatestBlocks(msgIdx: number, blocks: ValidatedCodeBlock[]) {
    const next = new Set(expandedBlocks);
    blocks.forEach((block, bi) => {
      const id = scriptIdentity(block);
      const key = `${msgIdx}-${bi}`;
      if (!id) {
        next.add(key);
        return;
      }
      for (const [iStr, list] of Object.entries(replyCodeBlocks)) {
        list.forEach((b, bix) => {
          const k = `${iStr}-${bix}`;
          if (k !== key && scriptIdentity(b) === id) next.delete(k);
        });
      }
      if (isUnchangedFromPrevious(block, msgIdx, bi)) next.delete(key);
      else next.add(key);
    });
    expandedBlocks = next;
  }

  function captureSnapshot() {
    return {
      messages: messages.map((m) => ({ ...m })),
      replyCodeBlocks: structuredClone(replyCodeBlocks),
      targetFiles: { ...targetFiles },
      expandedBlocks: [...expandedBlocks],
      input
    };
  }

  function truncateFrom(index: number) {
    messages = messages.slice(0, index);
    Object.keys(replyCodeBlocks).forEach((k) => {
      if (parseInt(k, 10) >= index) delete replyCodeBlocks[parseInt(k, 10)];
    });
    replyCodeBlocks = { ...replyCodeBlocks };
    Object.keys(targetFiles).forEach((k) => {
      const msgIdx = parseInt(k.split("-")[0] ?? "", 10);
      if (!Number.isNaN(msgIdx) && msgIdx >= index) delete targetFiles[k];
    });
    targetFiles = { ...targetFiles };
    expandedBlocks = new Set([...expandedBlocks].filter((k) => {
      const msgIdx = parseInt(k.split("-")[0] ?? "", 10);
      return Number.isNaN(msgIdx) || msgIdx < index;
    }));
  }

  function focusComposer() {
    queueMicrotask(() => {
      const el = document.getElementById("dst-chat-input") as HTMLInputElement | null;
      el?.focus();
      el?.scrollIntoView({ block: "nearest" });
    });
  }

  /** 回退：截断该条及之后，内容进入发送框，可取消恢复 */
  function rewindTo(index: number, content: string) {
    if (!draftSnapshot) draftSnapshot = captureSnapshot();
    draftMode = "rewind";
    truncateFrom(index);
    input = content;
    focusComposer();
  }

  /** 编辑：同回退，语义上强调改写后重发 */
  function editUserMessage(index: number, content: string) {
    if (!draftSnapshot) draftSnapshot = captureSnapshot();
    draftMode = "edit";
    truncateFrom(index);
    input = content;
    focusComposer();
  }

  function cancelDraft() {
    if (!draftSnapshot) {
      draftMode = null;
      input = "";
      return;
    }
    messages = draftSnapshot.messages;
    replyCodeBlocks = draftSnapshot.replyCodeBlocks;
    targetFiles = draftSnapshot.targetFiles;
    expandedBlocks = new Set(draftSnapshot.expandedBlocks);
    input = draftSnapshot.input;
    draftSnapshot = null;
    draftMode = null;
  }

  function clearDraftState() {
    draftSnapshot = null;
    draftMode = null;
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
            <div class="flex gap-2">
              <button
                type="button"
                class="text-xs text-slate-400 hover:text-cyan-300 disabled:opacity-40 underline-offset-2 hover:underline"
                disabled={loading}
                onclick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  editUserMessage(i, m.content);
                }}>编辑</button>
              <button
                type="button"
                class="text-xs text-slate-400 hover:text-amber-300 disabled:opacity-40 underline-offset-2 hover:underline"
                disabled={loading}
                onclick={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  rewindTo(i, m.content);
                }}>回退到此</button>
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
                {@const latest = isLatestScriptBlock(block, key)}
                {@const unchanged = latest && isUnchangedFromPrevious(block, i, bi)}
                {@const title = block.functions.join(", ") || "代码块"}
                {#if !latest}
                  <!-- 同命令旧稿：折叠为一行，避免多次优化堆叠编辑框 -->
                  <div class="border border-slate-800/80 rounded-lg overflow-hidden opacity-70">
                    <div class="flex items-center justify-between px-3 py-1.5 bg-slate-900/50 gap-2">
                      <div class="flex items-center gap-2 text-xs min-w-0">
                        <span class="text-cyan-300/70 font-mono truncate">{title}</span>
                        <span class="text-slate-500 shrink-0">已有更新版本</span>
                      </div>
                      <button
                        type="button"
                        class="text-xs text-slate-500 hover:text-slate-300 shrink-0"
                        onclick={() => toggleBlock(key)}>
                        {expandedBlocks.has(key) ? "收起旧稿" : "查看旧稿"}
                      </button>
                    </div>
                    {#if expandedBlocks.has(key)}
                      <pre class="px-3 py-2 text-xs text-slate-500 overflow-x-auto whitespace-pre-wrap font-mono bg-slate-950/40 border-t border-slate-800">{block.code}</pre>
                    {/if}
                  </div>
                {:else if unchanged}
                  <div class="border border-slate-800 rounded-lg px-3 py-1.5 flex items-center justify-between gap-2 bg-slate-900/40">
                    <div class="flex items-center gap-2 text-xs min-w-0">
                      <span class="text-cyan-300 font-mono truncate">{title}</span>
                      <span class="text-slate-500">与上一版相同 · {block.syntax_ok ? "语法✓" : "语法✗"} · {block.safety_ok ? "安全✓" : "安全✗"}</span>
                    </div>
                    <button
                      type="button"
                      class="text-xs text-slate-500 hover:text-slate-300 shrink-0"
                      onclick={() => toggleBlock(key)}>
                      {expandedBlocks.has(key) ? "收起" : "查看"}
                    </button>
                  </div>
                  {#if expandedBlocks.has(key)}
                    <pre class="mt-0 border border-t-0 border-slate-800 rounded-b-lg px-3 py-2 text-xs text-slate-400 overflow-x-auto whitespace-pre-wrap font-mono bg-slate-950/40">{block.code}</pre>
                  {/if}
                {:else}
                  <div class="border border-slate-700 rounded-lg overflow-hidden">
                    <div
                      class="flex items-center justify-between px-3 py-1.5 bg-slate-800/80 cursor-pointer hover:bg-slate-700/60"
                      role="button"
                      tabindex="0"
                      onclick={() => toggleBlock(key)}
                      onkeydown={(e) => e.key === "Enter" && toggleBlock(key)}>
                      <div class="flex items-center gap-2 text-xs">
                        <span class="text-slate-400 font-mono">{expandedBlocks.has(key) ? "▾" : "▸"}</span>
                        <span class="text-cyan-300 font-mono">{title}</span>
                        <span class="text-emerald-500/80">最新</span>
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
                {/if}
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

  <div class="p-3 border-t border-slate-700 space-y-2">
    {#if draftMode}
      <p class="text-xs text-amber-300/90 px-1">
        {draftMode === "edit" ? "正在编辑消息" : "已回退到此条"}，修改后发送将从此处继续
      </p>
    {/if}
    <div class="flex gap-2">
      <input
        id="dst-chat-input"
        bind:value={input}
        onkeydown={(e) => e.key === "Enter" && !e.shiftKey && (e.preventDefault(), send())}
        placeholder={draftMode ? "编辑后发送，或点取消恢复…" : "继续追问，或描述新命令…"}
        class="flex-1 px-3 py-2 text-sm bg-slate-800 border rounded-lg text-slate-200 focus:outline-none {draftMode
          ? 'border-amber-600/70 focus:border-amber-500'
          : 'border-slate-700 focus:border-cyan-600'}"
        disabled={loading || !profileId} />
      {#if draftMode}
        <button
          type="button"
          class="px-3 py-2 text-sm bg-slate-700 hover:bg-slate-600 rounded-lg disabled:opacity-50 transition-colors shrink-0"
          onclick={cancelDraft}
          disabled={loading}
          title="恢复编辑/回退前的对话">
          取消
        </button>
      {/if}
      <button
        class="px-4 py-2 text-sm bg-cyan-600 hover:bg-cyan-500 rounded-lg disabled:opacity-50 transition-colors shrink-0"
        onclick={send}
        disabled={loading || !input.trim() || !profileId}>
        发送
      </button>
    </div>
  </div>
</div>
