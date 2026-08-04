<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AiProfile } from "../api";

  let profiles = $state<AiProfile[]>([]);
  let defaultId = $state<string>("");
  let loading = $state(true);
  let errMsg = $state<string | null>(null);
  let successMsg = $state<string | null>(null);

  let showDialog = $state(false);
  let editing = $state<AiProfile | null>(null);
  let newKey = $state("");
  let testing = $state(false);
  let testOk = $state(false);
  let testMsg = $state("");

  async function load() {
    loading = true;
    errMsg = null;
    try {
      const meta = await api.getAiProfilesMeta();
      profiles = meta.profiles;
      defaultId = meta.default_profile_id ?? profiles[0]?.id ?? "";
    } catch (e) {
      errMsg = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function openAdd() {
    editing = {
      id: `p-${Date.now()}`,
      name: "",
      protocol: "openai",
      base_url: "https://api.openai.com/v1",
      model: "gpt-4o-mini",
      temperature: 0.7,
      max_tokens: 2048,
      key_configured: false
    };
    newKey = "";
    testOk = false;
    testMsg = "";
    showDialog = true;
  }

  function openEdit(p: AiProfile) {
    editing = { ...p };
    newKey = "";
    testOk = false;
    testMsg = "";
    showDialog = true;
  }

  async function testConnection() {
    if (!editing) return;
    testing = true;
    testOk = false;
    testMsg = "";
    errMsg = null;
    try {
      await api.saveAiProfile(editing, newKey.trim() || undefined);
      const reply = await api.testAiProfile(editing.id);
      testOk = true;
      testMsg = reply.slice(0, 120);
    } catch (e) {
      testOk = false;
      testMsg = String(e);
    } finally {
      testing = false;
    }
  }

  async function saveProfile() {
    if (!editing || !editing.name.trim()) {
      errMsg = "配置名称不能为空";
      return;
    }
    if (!testOk) {
      errMsg = "请先测试连接成功后再保存";
      return;
    }
    try {
      await api.saveAiProfile(editing, newKey.trim() || undefined);
      if (defaultId === editing.id || profiles.length === 0) {
        await api.setDefaultAiProfile(editing.id);
        defaultId = editing.id;
      }
      showDialog = false;
      successMsg = "已保存配置";
      await load();
    } catch (e) {
      errMsg = String(e);
    }
  }

  async function remove(id: string) {
    if (!confirm("确认删除此 AI 配置？")) return;
    try {
      await api.deleteAiProfile(id);
      await load();
    } catch (e) {
      errMsg = String(e);
    }
  }

  async function setDefault(id: string) {
    try {
      await api.setDefaultAiProfile(id);
      defaultId = id;
    } catch (e) {
      errMsg = String(e);
    }
  }
</script>

<div class="p-5 max-w-3xl mx-auto">
  <div class="flex items-center justify-between mb-4">
    <h2 class="text-lg font-semibold text-cyan-300">AI 配置</h2>
    <button class="px-3 py-1.5 text-sm bg-cyan-600 hover:bg-cyan-500 rounded" onclick={openAdd}>+ 添加配置</button>
  </div>

  {#if errMsg}
    <div class="mb-3 p-2 text-xs bg-red-900/40 border border-red-700 text-red-200 rounded">{errMsg}</div>
  {/if}
  {#if successMsg}
    <div class="mb-3 p-2 text-xs bg-green-900/40 border border-green-700 text-green-200 rounded">{successMsg}</div>
  {/if}

  {#if loading}
    <div class="space-y-2">
      <div class="h-10 bg-slate-800/60 rounded animate-pulse"></div>
      <div class="h-10 bg-slate-800/60 rounded animate-pulse"></div>
    </div>
  {:else if profiles.length === 0}
    <p class="text-sm text-slate-500">暂无配置，请点击「添加配置」。</p>
  {:else}
    <ul class="space-y-2">
      {#each profiles as p (p.id)}
        <li class="bg-slate-800/60 border border-slate-700 rounded p-3 flex items-center justify-between gap-3">
          <div>
            <div class="text-sm text-slate-200 font-medium">
              {p.name}
              {#if defaultId === p.id}<span class="text-xs text-cyan-400 ml-2">默认</span>{/if}
            </div>
            <div class="text-xs text-slate-400 mt-0.5">
              {p.protocol} · {p.model} · {p.key_configured ? "Key 已配置" : "Key 未配置"}
            </div>
          </div>
          <div class="flex gap-2 shrink-0">
            {#if defaultId !== p.id}
              <button class="text-xs text-slate-400 hover:text-cyan-300" onclick={() => setDefault(p.id)}>设为默认</button>
            {/if}
            <button class="text-xs text-cyan-400 hover:text-cyan-200" onclick={() => openEdit(p)}>编辑</button>
            <button class="text-xs text-red-400 hover:text-red-300" onclick={() => remove(p.id)}>删除</button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

{#if showDialog && editing}
  <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4">
    <div class="bg-slate-900 border border-slate-700 rounded-lg w-full max-w-lg p-5 max-h-[90vh] overflow-y-auto">
      <h3 class="text-base font-semibold text-cyan-300 mb-3">{editing.name ? "编辑配置" : "添加配置"}</h3>
      <div class="space-y-3">
        <label class="block">
          <span class="text-xs text-slate-400">名称</span>
          <input bind:value={editing.name} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded" />
        </label>
        <label class="block">
          <span class="text-xs text-slate-400">协议</span>
          <select bind:value={editing.protocol} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded">
            <option value="openai">OpenAI 兼容</option>
            <option value="anthropic">Anthropic</option>
          </select>
        </label>
        <label class="block">
          <span class="text-xs text-slate-400">Base URL</span>
          <input bind:value={editing.base_url} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded font-mono" />
        </label>
        <label class="block">
          <span class="text-xs text-slate-400">模型</span>
          <input bind:value={editing.model} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded font-mono" />
        </label>
        <label class="block">
          <span class="text-xs text-slate-400">API Key {editing.key_configured ? "(已配置，留空不修改)" : ""}</span>
          <input type="password" bind:value={newKey} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded font-mono" />
        </label>
        {#if testMsg}
          <div class="p-2 text-xs rounded border {testOk ? 'border-green-700 bg-green-900/30 text-green-200' : 'border-red-700 bg-red-900/30 text-red-200'}">
            {testOk ? "连接成功：" : "连接失败："}{testMsg}
          </div>
        {/if}
        <div class="flex gap-2 pt-2">
          <button class="px-3 py-1.5 text-sm bg-slate-700 hover:bg-slate-600 rounded" onclick={testConnection} disabled={testing}>
            {testing ? "测试中…" : "测试连接"}
          </button>
          <button class="px-3 py-1.5 text-sm bg-cyan-600 hover:bg-cyan-500 rounded disabled:opacity-50" onclick={saveProfile} disabled={!testOk}>保存</button>
          <button class="px-3 py-1.5 text-sm bg-slate-700 hover:bg-slate-600 rounded" onclick={() => (showDialog = false)}>取消</button>
        </div>
      </div>
    </div>
  </div>
{/if}
