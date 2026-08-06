use crate::error::{DstError, DstResult};
use crate::process_util::{output_hidden, ps_base_args};
use crate::{ps_parser, safety, sync, workspace};
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FunctionDraft {
    pub name: String,
    pub synopsis: String,
    pub example: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionTestResult {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

fn ps_exe() -> DstResult<&'static str> {
    let mut cmd = Command::new("pwsh");
    cmd.arg("--version");
    if crate::process_util::output_hidden_ref(&mut cmd).is_ok() {
        Ok("pwsh")
    } else {
        Ok("powershell")
    }
}

fn run_ps_script(script: &str) -> DstResult<String> {
    let exe = ps_exe()?;
    let mut cmd = Command::new(exe);
    for arg in ps_base_args(exe) {
        cmd.arg(arg);
    }
    cmd.args(["-Command", script]);
    let output = output_hidden(cmd).map_err(|e| DstError::Other(format!("启动 PowerShell 失败：{e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let out = String::from_utf8_lossy(&output.stdout);
        return Err(DstError::Other(format!(
            "PowerShell 失败：{}\n{}",
            err.trim(),
            out.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn default_body(name: &str) -> String {
    format!(
        r#"    [CmdletBinding()] param()
    Write-DstInfo "TODO: 实现 {name}""#
    )
}

/// 生成完整函数块文本。
pub fn build_function_block(draft: &FunctionDraft) -> String {
    let body = draft
        .body
        .clone()
        .unwrap_or_else(|| default_body(&draft.name));
    format!(
        r#"function {name} {{
<#
.SYNOPSIS
{synopsis}
.EXAMPLE
{example}
#>
{body}
}}
"#,
        name = draft.name,
        synopsis = draft.synopsis,
        example = draft.example,
        body = body.trim_end()
    )
}

fn assert_safety_ok(report: crate::safety::SafetyReport) -> DstResult<()> {
    if !report.ok {
        return Err(DstError::SafetyBlocked(report.violations.join("; ")));
    }
    Ok(())
}

fn validate_public_fn_name(name: &str) -> DstResult<()> {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        Some(c) if c.is_ascii_uppercase() => {
            return Err(DstError::Other(format!(
                "「{name}」是内部辅助函数（大写开头），不能在命令列表中编辑/删除/测试；请只管理小写公共命令"
            )));
        }
        _ => return Err(DstError::Other("函数名须以小写字母开头".into())),
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) {
        return Err(DstError::Other("函数名仅允许小写字母与数字".into()));
    }
    Ok(())
}

fn category_rel(file_name: &str) -> DstResult<String> {
    let name = file_name
        .trim()
        .trim_start_matches("Public/")
        .trim_start_matches("Public\\");
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        return Err(DstError::Other("分类文件名非法".into()));
    }
    if !name.ends_with(".ps1") {
        return Err(DstError::Other("文件名必须以 .ps1 结尾".into()));
    }
    Ok(format!("Public/{name}"))
}

fn write_temp_ps1(content: &str) -> DstResult<std::path::PathBuf> {
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "dst-fn-{}-{}.ps1",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&[0xEF, 0xBB, 0xBF])?;
        f.write_all(content.as_bytes())?;
    }
    Ok(tmp)
}

/// 插入或更新分类文件中的函数，并重生成公共部分。
pub fn upsert_function(
    file_name: &str,
    draft: FunctionDraft,
) -> DstResult<()> {
    let rel = category_rel(file_name)?;
    validate_public_fn_name(&draft.name)?;
    let file_path = workspace::workspace_root().join(&rel);
    if !file_path.exists() {
        return Err(DstError::FileNotFound(rel));
    }

    let block = build_function_block(&draft);
    assert_safety_ok(safety::check(&block)?)?;

    let block_tmp = write_temp_ps1(&block)?;
    let path_escaped = file_path.to_string_lossy().replace('\'', "''");
    let block_path_escaped = block_tmp.to_string_lossy().replace('\'', "''");
    let name_escaped = draft.name.replace('\'', "''");

    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
$path = '{path_escaped}'
$raw = Get-Content -LiteralPath $path -Raw -Encoding UTF8
$newBlock = Get-Content -LiteralPath '{block_path_escaped}' -Raw -Encoding UTF8
$errors = $null; $tokens = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput($raw, [ref]$tokens, [ref]$errors)
if ($errors -and $errors.Count -gt 0) {{ throw ($errors | ForEach-Object {{ $_.Message }}) -join '; ' }}
$fnName = '{name_escaped}'
$fnAsts = $ast.FindAll({{ param($n,$d) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq $fnName }}, $true)
if ($fnAsts -and $fnAsts.Count -gt 0) {{
    $start = $fnAsts[0].Extent.StartOffset
    $end = $fnAsts[0].Extent.EndOffset
    $raw = $raw.Substring(0, $start) + $newBlock + $raw.Substring($end)
}} else {{
    if (-not $raw.EndsWith("`n")) {{ $raw += "`n" }}
    $raw += "`n" + $newBlock
}}
$enc = New-Object System.Text.UTF8Encoding $true
[System.IO.File]::WriteAllText($path, $raw, $enc)
"#,
    );

    run_ps_script(&script)?;
    let _ = std::fs::remove_file(&block_tmp);

    let content = workspace::read_file(&rel)?;
    assert_safety_ok(safety::check(&content)?)?;
    let parsed = ps_parser::parse_ps1(&content)?;
    sync::regenerate_with_parsed(file_name, Some(parsed))?;
    workspace::touch_last_sync()?;
    crate::install_mgr::spawn_sync_runtime_modules();
    Ok(())
}

/// 从分类文件删除函数。
pub fn delete_function(file_name: &str, func_name: &str) -> DstResult<()> {
    let rel = category_rel(file_name)?;
    validate_public_fn_name(func_name)?;
    let file_path = workspace::workspace_root().join(&rel);
    if !file_path.exists() {
        return Err(DstError::FileNotFound(rel));
    }
    let path_escaped = file_path.to_string_lossy().replace('\'', "''");
    let name_escaped = func_name.replace('\'', "''");
    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
$path = '{path_escaped}'
$raw = Get-Content -LiteralPath $path -Raw -Encoding UTF8
$errors = $null; $tokens = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput($raw, [ref]$tokens, [ref]$errors)
$fnAsts = $ast.FindAll({{ param($n,$d) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq '{name_escaped}' }}, $true)
if (-not $fnAsts -or $fnAsts.Count -eq 0) {{ throw "函数不存在：{name_escaped}" }}
$start = $fnAsts[0].Extent.StartOffset
$end = $fnAsts[0].Extent.EndOffset
$raw = $raw.Substring(0, $start) + $raw.Substring($end)
$raw = ($raw -replace "(\r?\n){{3,}}", "`n`n").TrimEnd() + "`n"
$enc = New-Object System.Text.UTF8Encoding $true
[System.IO.File]::WriteAllText($path, $raw, $enc)
"#,
    );
    run_ps_script(&script)?;
    let content = workspace::read_file(&rel)?;
    let parsed = ps_parser::parse_ps1(&content)?;
    sync::regenerate_with_parsed(file_name, Some(parsed))?;
    workspace::touch_last_sync()?;
    crate::install_mgr::spawn_sync_runtime_modules();
    Ok(())
}

/// 在隔离进程中测试函数（dot-source + 执行 example）。
pub fn test_function(file_name: &str, func_name: &str) -> DstResult<FunctionTestResult> {
    let rel = category_rel(file_name)?;
    validate_public_fn_name(func_name)?;
    let content = workspace::read_file(&rel)?;
    let parsed = ps_parser::parse_ps1(&content)?;
    let func = parsed
        .functions
        .iter()
        .find(|f| f.name == func_name)
        .ok_or_else(|| DstError::Other(format!("函数不存在：{func_name}")))?;
    let example = if func.first_example.trim().is_empty() {
        func_name.to_string()
    } else {
        func.first_example.clone()
    };
    assert_safety_ok(safety::check(&example)?)?;

    let common = workspace::read_file("Private/Common.ps1")?;
    let common_tmp = write_temp_ps1(&common)?;
    let content_tmp = write_temp_ps1(&content)?;
    let common_path = common_tmp.to_string_lossy().replace('\'', "''");
    let content_path = content_tmp.to_string_lossy().replace('\'', "''");
    let example_escaped = example.replace('\'', "''");

    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
. '{common_path}'
. '{content_path}'
$result = Invoke-Expression '{example_escaped}' 2>&1 | Out-String
Write-Output $result
"#,
    );

    let result = match run_ps_script(&script) {
        Ok(stdout) => FunctionTestResult {
            ok: true,
            stdout,
            stderr: String::new(),
        },
        Err(e) => FunctionTestResult {
            ok: false,
            stdout: String::new(),
            stderr: e.to_string(),
        },
    };
    let _ = std::fs::remove_file(&common_tmp);
    let _ = std::fs::remove_file(&content_tmp);
    Ok(result)
}

/// 将 AI 生成的代码块合并到指定分类（按函数 AST 合并）。
pub fn apply_code_to_category(
    file_name: &str,
    code: &str,
) -> DstResult<Vec<String>> {
    apply_code_to_category_with_options(file_name, code, false)
}

pub fn apply_code_to_category_with_options(
    file_name: &str,
    code: &str,
    danger_mode: bool,
) -> DstResult<Vec<String>> {
    let report = safety::check_with_options(code, danger_mode)?;
    if !report.ok {
        return Err(DstError::SafetyBlocked(report.violations.join("; ")));
    }
    // parse_ps1 已含语法错误检查，无需再 validate_syntax
    let parsed = ps_parser::parse_ps1(code)?;
    let names: Vec<String> = parsed
        .functions
        .iter()
        .filter(|f| f.name.chars().next().map(|c| c.is_lowercase()).unwrap_or(false))
        .map(|f| f.name.clone())
        .collect();
    if names.is_empty() {
        return Err(DstError::Other("代码块中无可导出的公共函数".into()));
    }

    let rel = category_rel(file_name)?;
    let target = workspace::workspace_root().join(&rel);
    if !target.exists() {
        return Err(DstError::FileNotFound(rel));
    }

    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "dst-merge-{}-{}.ps1",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&[0xEF, 0xBB, 0xBF])?;
        f.write_all(code.as_bytes())?;
    }

    let target_esc = target.to_string_lossy().replace('\'', "''");
    let source_esc = tmp.to_string_lossy().replace('\'', "''");
    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
$targetPath = '{target_esc}'
$sourcePath = '{source_esc}'
$targetRaw = Get-Content -LiteralPath $targetPath -Raw -Encoding UTF8
$sourceRaw = Get-Content -LiteralPath $sourcePath -Raw -Encoding UTF8
$te=$null;$tt=$null; $targetAst = [System.Management.Automation.Language.Parser]::ParseInput($targetRaw, [ref]$tt, [ref]$te)
$se=$null;$st=$null; $sourceAst = [System.Management.Automation.Language.Parser]::ParseInput($sourceRaw, [ref]$st, [ref]$se)
$srcFns = $sourceAst.FindAll({{ param($n,$d) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] }}, $true)
foreach ($fn in $srcFns) {{
    if ($fn.Name.Substring(0,1) -cmatch '[A-Z]') {{ continue }}
    $text = $fn.Extent.Text
    $existing = $targetAst.FindAll({{ param($n,$d) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] -and $n.Name -eq $fn.Name }}, $true)
    if ($existing -and $existing.Count -gt 0) {{
        $start = $existing[0].Extent.StartOffset
        $end = $existing[0].Extent.EndOffset
        $targetRaw = $targetRaw.Substring(0, $start) + $text + $targetRaw.Substring($end)
        $te=$null;$tt=$null; $targetAst = [System.Management.Automation.Language.Parser]::ParseInput($targetRaw, [ref]$tt, [ref]$te)
    }} else {{
        if (-not $targetRaw.EndsWith("`n")) {{ $targetRaw += "`n" }}
        $targetRaw += "`n" + $text + "`n"
        $te=$null;$tt=$null; $targetAst = [System.Management.Automation.Language.Parser]::ParseInput($targetRaw, [ref]$tt, [ref]$te)
    }}
}}
$enc = New-Object System.Text.UTF8Encoding $true
[System.IO.File]::WriteAllText($targetPath, $targetRaw, $enc)
"#,
    );
    run_ps_script(&script)?;
    let _ = std::fs::remove_file(&tmp);

    let content = workspace::read_file(&rel)?;
    assert_safety_ok(safety::check_with_options(&content, danger_mode)?)?;
    let file_parsed = ps_parser::parse_ps1(&content)?;
    sync::regenerate_with_parsed(file_name, Some(file_parsed))?;
    workspace::touch_last_sync()?;
    crate::install_mgr::spawn_sync_runtime_modules();
    Ok(names)
}
