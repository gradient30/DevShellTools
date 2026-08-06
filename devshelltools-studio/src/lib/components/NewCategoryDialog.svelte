<script lang="ts">
  import { api, type SafetyReport } from "../api";

  let {
    onCreate,
    onCancel
  }: {
    onCreate: (fileName: string, content: string, message: string) => void;
    onCancel: () => void;
  } = $props();

  let catName = $state("");
  let catTitle = $state("");
  let catDesc = $state("");
  let catAliases = $state("");
  let funcName = $state("");
  let funcSynopsis = $state("");
  let funcExample = $state("");
  let funcBody = $state("");
  let syntaxOk = $state<boolean | null>(null);
  let syntaxErr = $state<string | null>(null);
  let safetyReport = $state<SafetyReport | null>(null);
  let errMsg = $state<string | null>(null);

  function buildContent(): string {
    const aliasesLine = catAliases.trim() ? catAliases.trim() : "";
    return `<#!
@DST-Category
Name: ${catName}
Title: ${catTitle}
Description: ${catDesc}
Aliases: ${aliasesLine}
@DST-Category-End
#>

function ${funcName} {
<#
.SYNOPSIS
${funcSynopsis}
.EXAMPLE
${funcExample}
#>
    [CmdletBinding()]
    param()
${funcBody}
}
`;
  }

  async function validate() {
    syntaxOk = null;
    syntaxErr = null;
    safetyReport = null;
    errMsg = null;
    if (!catName.trim() || !catTitle.trim() || !funcName.trim()) {
      errMsg = "分类关键字、标题、函数名必填";
      return;
    }
    if (!/^[a-z][a-z0-9]*$/.test(catName)) {
      errMsg = "分类关键字只能含小写字母和数字，以字母开头";
      return;
    }
    if (!/^[a-zA-Z][a-zA-Z0-9]*$/.test(funcName)) {
      errMsg = "函数名只能含字母和数字，以字母开头";
      return;
    }
    const content = buildContent();
    try {
      await api.validatePsSyntax(content);
      syntaxOk = true;
      safetyReport = await api.safetyCheck(content);
    } catch (e) {
      syntaxOk = false;
      syntaxErr = String(e);
    }
  }

  function submit() {
    const fileName = `${catName}.ps1`;
    onCreate(fileName, buildContent(), `新建分类 ${catName}`);
  }

  let canSubmit = $derived(
    syntaxOk === true && (safetyReport?.ok ?? false) && !errMsg
  );
</script>

<div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50 p-4">
  <div class="bg-dst-surface border border-dst-border rounded-lg w-full max-w-2xl max-h-[90vh] overflow-y-auto p-5">
    <h2 class="text-lg font-semibold text-dst-accent mb-4">新建分类</h2>

    {#if errMsg}
      <div class="mb-3 p-2 text-xs bg-dst-danger-bg border border-dst-danger-border text-dst-danger-fg rounded">{errMsg}</div>
    {/if}

    <div class="grid grid-cols-2 gap-3 mb-3">
      <label class="block">
        <span class="text-xs text-dst-fg-muted">分类关键字（小写字母+数字）</span>
        <input bind:value={catName} placeholder="docker" class="mt-1 w-full px-2 py-1 text-sm bg-dst-elevated border border-dst-border rounded text-dst-fg" />
      </label>
      <label class="block">
        <span class="text-xs text-dst-fg-muted">中文标题</span>
        <input bind:value={catTitle} placeholder="Docker" class="mt-1 w-full px-2 py-1 text-sm bg-dst-elevated border border-dst-border rounded text-dst-fg" />
      </label>
      <label class="block col-span-2">
        <span class="text-xs text-dst-fg-muted">说明</span>
        <input bind:value={catDesc} placeholder="容器管理快捷命令" class="mt-1 w-full px-2 py-1 text-sm bg-dst-elevated border border-dst-border rounded text-dst-fg" />
      </label>
      <label class="block col-span-2">
        <span class="text-xs text-dst-fg-muted">别名（逗号分隔，可空）</span>
        <input bind:value={catAliases} placeholder="容器,container" class="mt-1 w-full px-2 py-1 text-sm bg-dst-elevated border border-dst-border rounded text-dst-fg" />
      </label>
    </div>

    <hr class="border-dst-border my-3" />

    <h3 class="text-sm font-semibold text-dst-fg mb-2">首个命令</h3>
    <div class="grid grid-cols-2 gap-3 mb-3">
      <label class="block">
        <span class="text-xs text-dst-fg-muted">函数名</span>
        <input bind:value={funcName} placeholder="dps" class="mt-1 w-full px-2 py-1 text-sm bg-dst-elevated border border-dst-border rounded text-dst-fg font-mono" />
      </label>
      <label class="block">
        <span class="text-xs text-dst-fg-muted">SYNOPSIS</span>
        <input bind:value={funcSynopsis} placeholder="列出运行中容器" class="mt-1 w-full px-2 py-1 text-sm bg-dst-elevated border border-dst-border rounded text-dst-fg" />
      </label>
      <label class="block col-span-2">
        <span class="text-xs text-dst-fg-muted">EXAMPLE</span>
        <input bind:value={funcExample} placeholder="dps" class="mt-1 w-full px-2 py-1 text-sm bg-dst-elevated border border-dst-border rounded text-dst-fg font-mono" />
      </label>
      <label class="block col-span-2">
        <span class="text-xs text-dst-fg-muted">函数体（PowerShell 代码）</span>
        <textarea bind:value={funcBody} rows="4" placeholder="docker ps" class="mt-1 w-full px-2 py-1 text-xs font-mono bg-dst-bg border border-dst-border rounded text-dst-fg" spellcheck="false"></textarea>
      </label>
    </div>

    <div class="flex gap-2 mb-3">
      <button class="px-3 py-1 text-sm bg-dst-muted hover:bg-dst-muted rounded" onclick={validate}>校验</button>
      <button class="px-3 py-1 text-sm bg-dst-accent text-dst-accent-fg hover:bg-dst-accent-hover rounded disabled:opacity-50" onclick={submit} disabled={!canSubmit}>创建</button>
      <button class="px-3 py-1 text-sm bg-dst-muted hover:bg-dst-muted rounded" onclick={onCancel}>取消</button>
    </div>

    {#if syntaxOk === false}
      <div class="p-2 text-xs bg-dst-danger-bg border border-dst-danger-border text-dst-danger-fg rounded">语法错误：{syntaxErr}</div>
    {/if}
    {#if syntaxOk === true}
      <div class="p-2 text-xs bg-dst-success-bg border border-dst-success text-dst-success-fg rounded">语法校验通过</div>
    {/if}
    {#if safetyReport && !safetyReport.ok}
      <div class="mt-1 p-2 text-xs bg-dst-danger-bg border border-dst-danger-border text-dst-danger-fg rounded">安全拦截：{safetyReport.violations.join("；")}</div>
    {/if}
    {#if safetyReport?.ok}
      <div class="mt-1 p-2 text-xs bg-dst-success-bg border border-dst-success text-dst-success-fg rounded">安全检查通过</div>
    {/if}
  </div>
</div>