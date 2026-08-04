<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AiPreset, type AiProfile, type AiProtocol } from "../api";

  let profiles = $state<AiProfile[]>([]);
  let presets = $state<AiPreset[]>([]);
  let defaultId = $state<string>("");
  let loading = $state(true);
  let errMsg = $state<string | null>(null);
  let successMsg = $state<string | null>(null);

  let showDialog = $state(false);
  let editing = $state<AiProfile | null>(null);
  let selectedPresetId = $state("custom");
  let newKey = $state("");
  let testing = $state(false);
  let testOk = $state(false);
  let testMsg = $state("");
  let endpointNote = $state("");
  let testingProfile = $state<string | null>(null);

  let fetchingModels = $state(false);
  let modelOptions = $state<string[]>([]);
  let showModelPicker = $state(false);
  let modelSearch = $state("");

  let filteredModels = $derived(
    modelOptions.filter((m) => m.toLowerCase().includes(modelSearch.trim().toLowerCase()))
  );

  async function load() {
    loading = true;
    errMsg = null;
    try {
      const [meta, presetList] = await Promise.all([api.getAiProfilesMeta(), api.listAiPresets()]);
      profiles = meta.profiles;
      defaultId = meta.default_profile_id ?? profiles[0]?.id ?? "";
      presets = presetList;
    } catch (e) {
      errMsg = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(load);

  function matchPresetId(p: AiProfile): string {
    const hit = presets.find((x) => x.base_url === p.base_url && x.protocol === p.protocol);
    return hit?.id ?? "custom";
  }

  function applyPreset(presetId: string) {
    if (!editing || presetId === "custom") {
      selectedPresetId = presetId;
      return;
    }
    const p = presets.find((x) => x.id === presetId);
    if (!p) return;
    selectedPresetId = presetId;
    editing.protocol = p.protocol;
    editing.base_url = p.base_url;
    editing.model = p.default_model;
    endpointNote = `已应用预设「${p.name}」`;
    testOk = false;
    testMsg = "";
  }

  async function onProtocolChange(next: AiProtocol) {
    if (!editing) return;
    const suggestion = await api.suggestAiEndpoint(next, editing.base_url);
    editing.protocol = suggestion.protocol;
    editing.base_url = suggestion.base_url;
    editing.model = suggestion.default_model;
    endpointNote = suggestion.note;
    selectedPresetId = matchPresetId(editing);
    testOk = false;
    testMsg = "";
  }

  function openAdd() {
    const p = presets[0];
    editing = {
      id: `p-${Date.now()}`,
      name: "",
      protocol: p?.protocol ?? "openai",
      base_url: p?.base_url ?? "https://api.openai.com/v1",
      model: p?.default_model ?? "gpt-4o-mini",
      temperature: 0.7,
      max_tokens: 2048,
      key_configured: false
    };
    selectedPresetId = p?.id ?? "custom";
    newKey = "";
    testOk = false;
    testMsg = "";
    endpointNote = "";
    modelOptions = [];
    showModelPicker = false;
    showDialog = true;
  }

  function openEdit(p: AiProfile) {
    editing = { ...p };
    selectedPresetId = matchPresetId(p);
    newKey = "";
    testOk = false;
    testMsg = "";
    endpointNote = "";
    modelOptions = [];
    showModelPicker = false;
    showDialog = true;
  }

  async function fetchModels() {
    if (!editing) return;
    fetchingModels = true;
    errMsg = null;
    try {
      if (newKey.trim()) {
        modelOptions = await api.fetchAiModelsPreview(
          editing.protocol,
          editing.base_url,
          newKey.trim()
        );
      } else if (editing.key_configured) {
        modelOptions = await api.fetchAiModels(editing.id);
      } else {
        errMsg = "请先填写 API Key";
        return;
      }
      showModelPicker = true;
      modelSearch = "";
      if (modelOptions.length > 0 && !modelOptions.includes(editing.model)) {
        editing.model = modelOptions[0];
      }
    } catch (e) {
      errMsg = String(e);
    } finally {
      fetchingModels = false;
    }
  }

  function pickModel(id: string) {
    if (editing) editing.model = id;
    showModelPicker = false;
    testOk = false;
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

  async function testProfile(id: string) {
    testingProfile = id;
    try {
      const reply = await api.testAiProfile(id);
      showToast(`测试通过：${reply.slice(0, 80)}`, "success", 4000);
    } catch (e) {
      showToast(`测试失败：${String(e).slice(0, 120)}`, "error", 5000);
    } finally {
      testingProfile = null;
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
            <button class="text-xs text-emerald-400 hover:text-emerald-200" onclick={() => testProfile(p.id)} disabled={testingProfile === p.id}>
              {testingProfile === p.id ? "测试中…" : "测试"}
            </button>
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
          <span class="text-xs text-slate-400">提供商预设</span>
          <select
            class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded"
            value={selectedPresetId}
            onchange={(e) => applyPreset((e.currentTarget as HTMLSelectElement).value)}>
            <option value="custom">自定义</option>
            {#each presets as p (p.id)}
              <option value={p.id}>{p.name}</option>
            {/each}
          </select>
        </label>
        <label class="block">
          <span class="text-xs text-slate-400">协议</span>
          <select
            class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded"
            value={editing.protocol}
            onchange={(e) => onProtocolChange((e.currentTarget as HTMLSelectElement).value as AiProtocol)}>
            <option value="openai">OpenAI 兼容</option>
            <option value="anthropic">Anthropic</option>
          </select>
        </label>
        <label class="block">
          <span class="text-xs text-slate-400">Base URL</span>
          <input bind:value={editing.base_url} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded font-mono" />
          {#if endpointNote}
            <p class="text-xs text-amber-300/90 mt-1">{endpointNote}</p>
          {/if}
        </label>
        <div class="block">
          <div class="flex items-center justify-between">
            <span class="text-xs text-slate-400">模型</span>
            <button
              type="button"
              class="text-xs text-cyan-400 hover:text-cyan-200 disabled:opacity-50"
              onclick={fetchModels}
              disabled={fetchingModels || (!editing.key_configured && !newKey.trim())}>
              {fetchingModels ? "拉取中…" : "拉取模型"}
            </button>
          </div>
          <input bind:value={editing.model} class="mt-1 w-full px-2 py-1.5 text-sm bg-slate-800 border border-slate-700 rounded font-mono" />
          {#if showModelPicker && modelOptions.length > 0}
            <div class="mt-2 border border-slate-700 rounded bg-slate-950/80 p-2">
              <input
                bind:value={modelSearch}
                placeholder="搜索模型…"
                class="w-full px-2 py-1 text-xs bg-slate-900 border border-slate-700 rounded mb-2" />
              <ul class="max-h-40 overflow-y-auto text-xs space-y-0.5">
                {#each filteredModels as m (m)}
                  <li>
                    <button type="button" class="w-full text-left px-2 py-1 rounded hover:bg-slate-800 font-mono text-slate-200" onclick={() => pickModel(m)}>
                      {m}
                    </button>
                  </li>
                {/each}
                {#if filteredModels.length === 0}
                  <li class="text-slate-500 px-2 py-1">无匹配模型</li>
                {/if}
              </ul>
            </div>
          {/if}
        </div>
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
