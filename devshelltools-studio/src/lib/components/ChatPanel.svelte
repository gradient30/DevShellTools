<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api, type AiProfile, type CategoryInfo, type ChatMessage, type ValidatedCodeBlock } from "../api";
  import { showToast } from "../stores/toast";

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
    onApplyCode: (code: string, fileName: string, dangerMode?: boolean) => Promise<void>;
    onOpenSettings: () => void;
  } = $props();

  /** 带稳定 id 的 UI 消息（发往 API 时剥掉 id） */
  type UiMessage = ChatMessage & { id: string };

  let profiles = $state<AiProfile[]>([]);
  let profileId = $state("");
  let messages = $state<UiMessage[]>([]);
  let input = $state("");
  let loading = $state(false);
  let errMsg = $state<string | null>(null);
  /** 本会话最高权限：输入 /danger 开启，/safe 关闭 */
  let dangerMode = $state(false);
  let applying = $state(false);
  let replyCodeBlocks = $state<Record<number, ValidatedCodeBlock[]>>({});
  let targetFiles = $state<Record<string, string>>({});
  let expandedBlocks = $state<Set<string>>(new Set());
  let lastAutoToken = $state(0);
  let profilesReady = $state(false);

  /** 就地编辑：正在编辑的用户消息 id；null 表示未在编辑 */
  let editingId = $state<string | null>(null);
  let editText = $state("");
  /** 回退后可「取消回退」恢复的快照 */
  let rewindBackup = $state<{
    messages: UiMessage[];
    replyCodeBlocks: Record<number, ValidatedCodeBlock[]>;
    targetFiles: Record<string, string>;
    expandedBlocks: string[];
    input: string;
  } | null>(null);

  let msgSeq = 0;
  function newId(role: string): string {
    msgSeq += 1;
    return `${role}-${msgSeq}-${Date.now()}`;
  }

  function toApiMessages(list: UiMessage[]): ChatMessage[] {
    return list.map(({ role, content }) => ({ role, content }));
  }

  async function loadProfiles(syncDefault = true) {
    const meta = await api.getAiProfilesMeta();
    profiles = meta.profiles;
    const defaultId = meta.default_profile_id;
    const defaultExists = !!defaultId && profiles.some((p) => p.id === defaultId);
    if (syncDefault && defaultExists) {
      profileId = defaultId!;
      return;
    }
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
    })();

    const onConfigChanged = () => {
      void loadProfiles(true).catch((e) => {
        errMsg = String(e);
      });
    };
    window.addEventListener("ai-config-changed", onConfigChanged);
    return () => window.removeEventListener("ai-config-changed", onConfigChanged);
  });

  // 仅响应「命令列表 → AI审阅」的 token，不与编辑/回退共享 effect，避免状态被冲掉
  $effect(() => {
    const token = autoSendToken;
    const ready = profilesReady;
    const pid = profileId;
    if (!ready || !pid) return;
    if (token <= 0 || token === lastAutoToken) return;
    lastAutoToken = token;
    const prompt = initialPrompt.trim();
    if (!prompt) return;
    editingId = null;
    editText = "";
    rewindBackup = null;
    replyCodeBlocks = {};
    targetFiles = {};
    expandedBlocks = new Set();
    input = "";
    void sendPrompt(prompt, { replaceAll: true });
  });

  function handleSessionCommand(raw: string): boolean {
    const cmd = raw.trim().toLowerCase();
    if (cmd === "/danger") {
      dangerMode = true;
      input = "";
      messages = [
        ...messages,
        { id: newId("user"), role: "user", content: "/danger" },
        {
          id: newId("assistant"),
          role: "assistant",
          content:
            "【危险模式已开启】本会话可生成/插入含 git reset --hard、force-push、真实 git clean 等破坏性命令。输入 /safe 可恢复默认红线。"
        }
      ];
      showToast("危险模式已开启", "info", 3500);
      return true;
    }
    if (cmd === "/safe") {
      dangerMode = false;
      input = "";
      messages = [
        ...messages,
        { id: newId("user"), role: "user", content: "/safe" },
        {
          id: newId("assistant"),
          role: "assistant",
          content: "已关闭危险模式，恢复默认安全红线。"
        }
      ];
      showToast("已恢复安全模式", "success", 2500);
      return true;
    }
    return false;
  }

  async function sendPrompt(text: string, opts?: { replaceAll?: boolean }) {
    const trimmed = text.trim();
    if (!trimmed || loading || !profileId) return;
    if (!opts?.replaceAll && handleSessionCommand(trimmed)) return;

    if (opts?.replaceAll) {
      messages = [{ id: newId("user"), role: "user", content: trimmed }];
    } else {
      messages = [...messages, { id: newId("user"), role: "user", content: trimmed }];
    }

    input = "";
    editingId = null;
    editText = "";
    rewindBackup = null;
    loading = true;
    errMsg = null;

    const apiMessages = toApiMessages(messages);
    try {
      const result = await api.aiChatWithValidation(apiMessages, profileId, dangerMode);
      const assistantIdx = messages.length;
      messages = [
        ...messages,
        { id: newId("assistant"), role: "assistant", content: result.reply }
      ];
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
        focusLatestBlocks(assistantIdx, result.code_blocks);
      }
    } catch (e) {
      const msg = String(e);
      if (!msg.includes("已停止生成")) errMsg = msg;
    } finally {
      loading = false;
    }
  }

  async function send() {
    await sendPrompt(input);
  }

  async function stopGeneration() {
    if (!loading) return;
    try {
      await api.aiCancelChat();
    } catch {
      /* ignore */
    }
  }

  function truncateFrom(index: number) {
    messages = messages.slice(0, index);
    const nextBlocks: Record<number, ValidatedCodeBlock[]> = {};
    Object.entries(replyCodeBlocks).forEach(([k, v]) => {
      const n = parseInt(k, 10);
      if (!Number.isNaN(n) && n < index) nextBlocks[n] = v;
    });
    replyCodeBlocks = nextBlocks;
    const nextTargets: Record<string, string> = {};
    Object.entries(targetFiles).forEach(([k, v]) => {
      const msgIdx = parseInt(k.split("-")[0] ?? "", 10);
      if (!Number.isNaN(msgIdx) && msgIdx < index) nextTargets[k] = v;
    });
    targetFiles = nextTargets;
    expandedBlocks = new Set(
      [...expandedBlocks].filter((k) => {
        const msgIdx = parseInt(k.split("-")[0] ?? "", 10);
        return Number.isNaN(msgIdx) || msgIdx < index;
      })
    );
  }

  /** 就地编辑：不删消息，在气泡内改写 */
  async function beginEdit(index: number) {
    if (loading) {
      showToast("请先停止当前生成，再编辑", "info", 2500);
      return;
    }
    const m = messages[index];
    if (!m || m.role !== "user") return;
    editingId = m.id;
    editText = m.content;
    rewindBackup = null;
    errMsg = null;
    showToast("已进入编辑，改完后点「重新发送」", "info", 2500);
    await tick();
    const el = document.getElementById(`dst-edit-${m.id}`) as HTMLTextAreaElement | null;
    el?.focus();
    el?.scrollIntoView({ block: "nearest" });
  }

  function cancelEdit() {
    editingId = null;
    editText = "";
  }

  /** 用编辑后的文本截断并重发 */
  async function confirmEditResend() {
    if (loading || !editingId) return;
    const idx = messages.findIndex((m) => m.id === editingId);
    if (idx < 0) {
      cancelEdit();
      return;
    }
    const text = editText.trim();
    if (!text) {
      showToast("内容不能为空", "error", 2000);
      return;
    }
    truncateFrom(idx);
    editingId = null;
    editText = "";
    showToast("正在按修改后的提示重新发送…", "info", 2000);
    await sendPrompt(text);
  }

  /** 回退到此：删掉该条及之后，原文放入底部输入框待重发 */
  async function rewindTo(index: number) {
    if (loading) {
      showToast("请先停止当前生成，再回退", "info", 2500);
      return;
    }
    const m = messages[index];
    if (!m || m.role !== "user") return;
    rewindBackup = {
      messages: messages.map((x) => ({ ...x })),
      replyCodeBlocks: structuredClone(replyCodeBlocks),
      targetFiles: { ...targetFiles },
      expandedBlocks: [...expandedBlocks],
      input
    };
    editingId = null;
    editText = "";
    truncateFrom(index);
    input = m.content;
    errMsg = null;
    showToast("已回退：修改底部内容后发送，或点「取消回退」", "info", 3500);
    await tick();
    const el = document.getElementById("dst-chat-input") as HTMLTextAreaElement | null;
    el?.focus();
    el?.scrollIntoView({ block: "nearest" });
  }

  function cancelRewind() {
    if (!rewindBackup) return;
    messages = rewindBackup.messages;
    replyCodeBlocks = rewindBackup.replyCodeBlocks;
    targetFiles = rewindBackup.targetFiles;
    expandedBlocks = new Set(rewindBackup.expandedBlocks);
    input = rewindBackup.input;
    rewindBackup = null;
    showToast("已取消回退，对话已恢复", "info", 2000);
  }

  async function apply(block: ValidatedCodeBlock, key: string) {
    if (!block.syntax_ok || !block.safety_ok) return;
    const fileName = targetFiles[key];
    if (!fileName) {
      errMsg = "请选择目标分类";
      return;
    }
    if (dangerMode) {
      const ok = confirm(
        "当前为危险模式，即将插入可能含破坏性操作的代码。确认继续？"
      );
      if (!ok) return;
    }
    applying = true;
    try {
      await onApplyCode(block.code, fileName, dangerMode);
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

  function scriptIdentity(block: ValidatedCodeBlock): string | null {
    if (block.functions.length === 0) return null;
    return [...block.functions].map((f) => f.toLowerCase()).sort().join(",");
  }

  function normalizeCode(code: string): string {
    return code.replace(/\r\n/g, "\n").trim();
  }

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

  function textWithoutCode(text: string): string {
    return text.replace(/```[\s\S]*?```/g, "").trim();
  }
</script>

<div class="h-full flex flex-col">
  <div class="px-4 py-2.5 border-b border-slate-700 flex items-center justify-between gap-2">
    <div class="flex items-center gap-2 min-w-0">
      <span class="text-lg">🤖</span>
      <h3 class="text-sm font-semibold text-cyan-300 shrink-0">AI 助手</h3>
      <select
        bind:value={profileId}
        class="text-xs bg-slate-800 border border-slate-700 rounded px-2 py-1 text-slate-200 max-w-[180px]"
        disabled={loading}>
        {#each profiles as p (p.id)}
          <option value={p.id}>{p.name} · {p.model}</option>
        {/each}
      </select>
    </div>
    <button type="button" class="text-xs text-slate-400 hover:text-slate-200 shrink-0" onclick={onOpenSettings}>
      配置
    </button>
  </div>

  {#if dangerMode}
    <div class="px-4 py-2 bg-red-950/80 border-b border-red-600 text-red-100 text-xs flex items-center justify-between gap-2">
      <span>⚠ 危险模式：本会话已放宽安全红线（reset --hard / force-push / 真实 clean 等）</span>
      <button
        type="button"
        class="shrink-0 px-2 py-0.5 rounded border border-red-500/60 hover:bg-red-900/50"
        onclick={() => {
          dangerMode = false;
          showToast("已恢复安全模式", "success", 2000);
        }}>
        /safe
      </button>
    </div>
  {/if}

  {#if errMsg}
    <div class="px-4 py-2 bg-red-900/40 border-b border-red-700 text-red-200 text-xs flex justify-between gap-2">
      <span class="break-words min-w-0">{errMsg}</span>
      <button type="button" class="text-red-300 shrink-0" onclick={() => (errMsg = null)}>×</button>
    </div>
  {/if}

  <div class="flex-1 overflow-y-auto px-4 py-4 space-y-4">
    {#if messages.length === 0 && !loading}
      <div class="flex flex-col items-center justify-center h-full text-center gap-3">
        <div class="text-4xl">🤖</div>
        <p class="text-sm text-slate-400">可直接提问，或从命令列表点「AI审阅」</p>
        <p class="text-xs text-slate-600">审阅流程：检查问题 → 优化建议 → 可新增命令建议；校验通过后可插入分类</p>
        <p class="text-xs text-slate-600">需要破坏性 git 能力时输入 <code class="text-amber-400/90">/danger</code>，用 <code class="text-slate-400">/safe</code> 关闭</p>
      </div>
    {/if}

    {#each messages as m, i (m.id)}
      {#if m.role === "user"}
        <div class="flex justify-end gap-2" data-msg-id={m.id}>
          <div class="flex flex-col items-end gap-1.5 max-w-[85%] min-w-0 w-full">
            {#if editingId === m.id}
              <!-- 就地编辑：气泡内改写，不依赖底部输入框 -->
              <div class="w-full rounded-lg border-2 border-amber-500/80 bg-slate-900 p-2 space-y-2">
                <p class="text-xs text-amber-300 px-1">编辑此条提示词，确认后将删除其后的回复并重新发送</p>
                <textarea
                  id="dst-edit-{m.id}"
                  bind:value={editText}
                  rows={8}
                  class="w-full px-3 py-2 text-sm bg-slate-950 border border-amber-700/50 rounded-lg text-slate-100 focus:outline-none focus:border-amber-500 resize-y min-h-[8rem]"
                  disabled={loading}></textarea>
                <div class="flex gap-2 justify-end">
                  <button
                    type="button"
                    class="px-3 py-1.5 text-xs bg-slate-700 hover:bg-slate-600 rounded"
                    onclick={cancelEdit}
                    disabled={loading}>
                    取消
                  </button>
                  <button
                    type="button"
                    class="px-3 py-1.5 text-xs bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50"
                    onclick={confirmEditResend}
                    disabled={loading || !editText.trim()}>
                    重新发送
                  </button>
                </div>
              </div>
            {:else}
              <div class="bg-cyan-700/40 rounded-lg rounded-tr-sm px-3 py-2 text-sm text-cyan-50 whitespace-pre-wrap break-words max-h-64 overflow-y-auto">
                {m.content}
              </div>
              <div class="flex gap-2 items-center relative z-10">
                <button
                  type="button"
                  class="px-2.5 py-1 text-xs rounded bg-slate-800 border border-cyan-600/60 text-cyan-300 hover:bg-cyan-900/40 hover:border-cyan-400 disabled:opacity-40"
                  disabled={loading || editingId !== null}
                  onclick={() => beginEdit(i)}>
                  编辑
                </button>
                <button
                  type="button"
                  class="px-2.5 py-1 text-xs rounded bg-slate-800 border border-amber-600/60 text-amber-300 hover:bg-amber-900/40 hover:border-amber-400 disabled:opacity-40"
                  disabled={loading || editingId !== null}
                  onclick={() => rewindTo(i)}>
                  回退到此
                </button>
              </div>
            {/if}
          </div>
          <div class="w-8 h-8 rounded-full bg-cyan-800/60 border border-cyan-700 flex items-center justify-center text-sm shrink-0">
            👤
          </div>
        </div>
      {:else}
        <div class="flex justify-start gap-2">
          <div class="w-8 h-8 rounded-full bg-emerald-800/60 border border-emerald-700 flex items-center justify-center text-sm shrink-0">
            🤖
          </div>
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
                  <div class="border border-slate-800/80 rounded-lg overflow-hidden opacity-70">
                    <div class="flex items-center justify-between px-3 py-1.5 bg-slate-900/50 gap-2">
                      <div class="flex items-center gap-2 text-xs min-w-0">
                        <span class="text-cyan-300/70 font-mono truncate">{title}</span>
                        <span class="text-slate-500 shrink-0">已有更新版本</span>
                      </div>
                      <button type="button" class="text-xs text-slate-500 hover:text-slate-300 shrink-0" onclick={() => toggleBlock(key)}>
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
                    <button type="button" class="text-xs text-slate-500 hover:text-slate-300 shrink-0" onclick={() => toggleBlock(key)}>
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
                            type="button"
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
        <div class="w-8 h-8 rounded-full bg-emerald-800/60 border border-emerald-700 flex items-center justify-center text-sm shrink-0">
          🤖
        </div>
        <div class="flex items-center gap-3 bg-slate-800/60 rounded-lg rounded-tl-sm px-3 py-2">
          <div class="flex items-center gap-2">
            <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 0ms"></span>
            <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 150ms"></span>
            <span class="inline-block w-2 h-2 bg-emerald-400 rounded-full animate-bounce" style="animation-delay: 300ms"></span>
            <span class="text-xs text-slate-400 ml-1">正在生成…</span>
          </div>
          <button
            type="button"
            class="px-2 py-0.5 text-xs rounded border border-red-700/70 text-red-300 hover:bg-red-900/40"
            onclick={stopGeneration}>
            停止
          </button>
        </div>
      </div>
    {/if}
  </div>

  <div class="p-3 border-t border-slate-700 space-y-2 shrink-0">
    {#if rewindBackup}
      <div class="flex items-center justify-between gap-2 px-1">
        <p class="text-xs text-amber-300">已回退：编辑下方内容后发送，将从该条继续</p>
        <button type="button" class="text-xs text-slate-400 hover:text-slate-200 shrink-0" onclick={cancelRewind} disabled={loading}>
          取消回退
        </button>
      </div>
    {/if}
    {#if editingId}
      <p class="text-xs text-amber-300/80 px-1">正在编辑上方消息 — 请在气泡内点「重新发送」</p>
    {/if}
    <div class="flex gap-2 items-end">
      <textarea
        id="dst-chat-input"
        bind:value={input}
        rows={rewindBackup ? 5 : 2}
        onkeydown={(e) => {
          if (e.key === "Enter" && !e.shiftKey && !loading && !editingId) {
            e.preventDefault();
            void send();
          }
        }}
        placeholder={rewindBackup
          ? "回退后可修改，然后发送…"
          : dangerMode
            ? "危险模式已开…（/safe 关闭）"
            : "继续追问…（/danger 放宽红线 · Shift+Enter 换行）"}
        class="flex-1 px-3 py-2 text-sm bg-slate-800 border rounded-lg text-slate-200 focus:outline-none resize-y min-h-[2.5rem] max-h-48 {rewindBackup
          ? 'border-amber-600/70 focus:border-amber-500'
          : dangerMode
            ? 'border-red-600/70 focus:border-red-500'
            : 'border-slate-700 focus:border-cyan-600'}"
        disabled={loading || !profileId || editingId !== null}></textarea>
      {#if loading}
        <button
          type="button"
          class="px-4 py-2 text-sm bg-red-800/80 hover:bg-red-700 rounded-lg shrink-0"
          onclick={stopGeneration}>
          停止
        </button>
      {:else}
        <button
          type="button"
          class="px-4 py-2 text-sm bg-cyan-600 hover:bg-cyan-500 rounded-lg disabled:opacity-50 shrink-0"
          onclick={send}
          disabled={!input.trim() || !profileId || editingId !== null}>
          发送
        </button>
      {/if}
    </div>
  </div>
</div>
