<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AiConfig, type AiKeyStatus } from "../api";

  let {
    onClose
  }: {
    onClose: () => void;
  } = $props();

  let config = $state<AiConfig>({
    protocol: "openai",
    base_url: "https://api.openai.com/v1",
    model: "gpt-4o-mini",
    temperature: 0.7,
    max_tokens: 2048
  });
  let keyStatus = $state<AiKeyStatus>({ configured: false, masked: "" });
  let newKey = $state("");
  let saving = $state(false);
  let saved = $state(false);
  let errMsg = $state<string | null>(null);

  onMount(() => {
    loadConfig();
  });

  async function loadConfig() {
    try {
      config = await api.getAiConfig();
      keyStatus = await api.getAiKeyStatus();
    } catch (e) {
      errMsg = String(e);
    }
  }

  async function save() {
    saving = true;
    saved = false;
    errMsg = null;
    try {
      await api.saveAiConfig(config);
      if (newKey.trim()) {
        await api.saveAiKey(newKey.trim());
        newKey = "";
        keyStatus = await api.getAiKeyStatus();
      }
      saved = true;
      setTimeout(() => (saved = false), 2000);
    } catch (e) {
      errMsg = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4">
  <div class="bg-slate-900 border border-slate-700 rounded-lg w-full max-w-lg p-5">
    <div class="flex justify-between items-center mb-4">
      <h2 class="text-lg font-semibold text-cyan-300">AI 设置</h2>
      <button class="text-slate-400 hover:text-slate-200" onclick={onClose}>×</button>
    </div>

    {#if errMsg}
      <div class="mb-3 p-2 text-xs bg-red-900/40 border border-red-700 text-red-200 rounded">{errMsg}</div>
    {/if}
    {#if saved}
      <div class="mb-3 p-2 text-xs bg-green-900/40 border border-green-700 text-green-200 rounded">已保存</div>
    {/if}

    <div class="space-y-3">
      <label class="block">
        <span class="text-xs text-slate-400">协议</span>
        <select bind:value={config.protocol} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded text-slate-200">
          <option value="openai">OpenAI 兼容</option>
          <option value="anthropic">Anthropic</option>
        </select>
      </label>

      <label class="block">
        <span class="text-xs text-slate-400">Base URL</span>
        <input bind:value={config.base_url} placeholder="https://api.openai.com/v1" class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded text-slate-200 font-mono" />
      </label>

      <label class="block">
        <span class="text-xs text-slate-400">模型</span>
        <input bind:value={config.model} placeholder="gpt-4o-mini" class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded text-slate-200 font-mono" />
      </label>

      <div class="grid grid-cols-2 gap-3">
        <label class="block">
          <span class="text-xs text-slate-400">Temperature</span>
          <input type="number" min="0" max="2" step="0.1" bind:value={config.temperature} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded text-slate-200" />
        </label>
        <label class="block">
          <span class="text-xs text-slate-400">Max Tokens</span>
          <input type="number" min="1" max="8192" step="1" bind:value={config.max_tokens} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded text-slate-200" />
        </label>
      </div>

      <hr class="border-slate-700" />

      <label class="block">
        <span class="text-xs text-slate-400">API Key {keyStatus.configured ? `（当前：${keyStatus.masked}）` : "（未配置）"}</span>
        <input type="password" bind:value={newKey} placeholder="留空则不修改" class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded text-slate-200 font-mono" />
      </label>

      <div class="flex gap-2 pt-2">
        <button class="px-4 py-1.5 text-sm bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50" onclick={save} disabled={saving}>
          {saving ? "保存中…" : "保存"}
        </button>
        <button class="px-4 py-1.5 text-sm bg-slate-700 hover:bg-slate-600 rounded" onclick={onClose}>关闭</button>
      </div>

      <p class="text-xs text-slate-500 mt-2">
        Key 存储在 <code>.studio/ai_key.txt</code>（便携优先，不入库）。安全边界规则固化在后端，前端不可放宽。
      </p>
    </div>
  </div>
</div>