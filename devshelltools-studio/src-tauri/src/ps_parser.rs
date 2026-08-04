use crate::error::{DstError, DstResult};
use crate::process_util::{output_hidden, ps_base_args};
use serde::{Deserialize, Serialize};
use std::process::Command;

/// 一个 PowerShell 函数的元信息（从 AST 提取）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PsFunction {
    pub name: String,
    pub synopsis: String,
    #[serde(rename = "first_example")]
    pub first_example: String,
}

/// 一个分类的元信息（从 @DST-Category 块提取）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryMeta {
    pub name: String,
    pub title: String,
    pub description: String,
    #[serde(deserialize_with = "deserialize_string_or_array")]
    pub aliases: Vec<String>,
}

/// 解析一个 .ps1 文件的结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPsFile {
    pub category: Option<CategoryMeta>,
    pub functions: Vec<PsFunction>,
    #[serde(rename = "parseErrors")]
    pub parse_errors: Vec<String>,
}

/// 兼容 PS5.1 ConvertTo-Json 的 string-or-array 行为：
/// 单元素数组被序列化成字符串，需还原成 Vec。
fn deserialize_string_or_array<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrArr {
        Single(String),
        Multi(Vec<String>),
    }
    match StrOrArr::deserialize(deserializer)? {
        StrOrArr::Single(s) => {
            if s.trim().is_empty() {
                Ok(vec![])
            } else {
                Ok(vec![s])
            }
        }
        StrOrArr::Multi(v) => Ok(v),
    }
}

/// 调用 powershell.exe 解析 .ps1 文本，返回结构化结果。
/// 把 content 通过临时文件传递，避免 stdin 管道死锁。
pub fn parse_ps1(content: &str) -> DstResult<ParsedPsFile> {
    let content = content.strip_prefix('\u{FEFF}').unwrap_or(content);
    let exe = which_powershell()?;
    // 写临时 .ps1 文件（带 BOM 让 PS5.1 正确识别 UTF-8）
    let mut tmp_path = std::env::temp_dir();
    tmp_path.push(format!(
        "dst-parse-{}-{}.ps1",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    {
        use std::io::Write;
        let mut f = std::fs::File::create(&tmp_path)
            .map_err(|e| DstError::PsParse(format!("创建临时文件失败：{e}")))?;
        // 写 UTF-8 BOM + 内容
        f.write_all(&[0xEF, 0xBB, 0xBF])
            .map_err(|e| DstError::PsParse(format!("写 BOM 失败：{e}")))?;
        f.write_all(content.as_bytes())
            .map_err(|e| DstError::PsParse(format!("写内容失败：{e}")))?;
    }

    // 构造解析脚本：读取临时文件内容并解析
    let tmp_path_str = tmp_path.to_string_lossy().replace('\'', "''");
    let script = format!(
        r#"$ErrorActionPreference = 'Stop'
$raw = Get-Content -LiteralPath '{tmp_path_str}' -Raw -Encoding UTF8
$errors = $null
$tokens = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput($raw, [ref]$tokens, [ref]$errors)
$result = [ordered]@{{ category = $null; functions = @(); parseErrors = @() }}
foreach ($e in $errors) {{ $result.parseErrors += $e.Extent.StartLineNumber.ToString() + ':' + $e.Message }}
if ($null -ne $ast) {{
    $text = $ast.ToString()
    $m = [regex]::Match($text, '(?s)@DST-Category\s*\r?\n(.*?)\r?\n@DST-Category-End')
    if ($m.Success) {{
        $block = $m.Groups[1].Value
        $name = ''; $title = ''; $description = ''; $aliases = @()
        foreach ($line in $block -split "`r?`n") {{
            if ($line -match '^\s*Name:\s*(.+?)\s*$') {{ $name = $Matches[1] }}
            elseif ($line -match '^\s*Title:\s*(.+?)\s*$') {{ $title = $Matches[1] }}
            elseif ($line -match '^\s*Description:\s*(.+?)\s*$') {{ $description = $Matches[1] }}
            elseif ($line -match '^\s*Aliases:\s*(.*)$') {{
                $a = $Matches[1].Trim()
                if ($a) {{ $aliases = ($a -split ',') | ForEach-Object {{ $_.Trim() }} | Where-Object {{ $_ }} }}
            }}
        }}
        if ($name) {{ $result.category = [PSCustomObject]@{{ name = $name; title = $title; description = $description; aliases = $aliases }} }}
    }}
    $fnAsts = $ast.FindAll({{ param($n, $_) $n -is [System.Management.Automation.Language.FunctionDefinitionAst] }}, $true)
    foreach ($fnAst in $fnAsts) {{
        $synopsis = ''; $example = ''
        $help = $fnAst.GetHelpContent()
        if ($help.Synopsis) {{ $synopsis = $help.Synopsis.Trim() }}
        if ($help.Examples -and $help.Examples.Count -gt 0) {{ $example = ($help.Examples[0] -split "`r?`n")[0].Trim() }}
        $result.functions += [PSCustomObject]@{{ name = $fnAst.Name; synopsis = $synopsis; first_example = $example }}
    }}
}}
$result | ConvertTo-Json -Depth 5 -Compress
"#
    );

    let mut cmd = Command::new(exe);
    for arg in ps_base_args(exe) {
        cmd.arg(arg);
    }
    cmd.args(["-Command", &script]);
    let output = output_hidden(cmd)
        .map_err(|e| DstError::PsParse(format!("启动 powershell 失败：{e}")))?;

    // 清理临时文件
    let _ = std::fs::remove_file(&tmp_path);

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(DstError::PsParse(format!(
            "powershell 退出码 {}：{}",
            output.status,
            err.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.starts_with('{'))
        .ok_or_else(|| DstError::PsParse(format!("未找到 JSON 输出，stdout: {}", stdout.trim())))?;

    let parsed: ParsedPsFile =
        serde_json::from_str(json_line).map_err(|e| DstError::PsParse(format!("反序列化失败：{e}")))?;

    if !parsed.parse_errors.is_empty() {
        return Err(DstError::PsParse(format!(
            "语法错误：{}",
            parsed.parse_errors.join("; ")
        )));
    }
    Ok(parsed)
}

fn which_powershell() -> DstResult<&'static str> {
    // 优先 pwsh（PS7），回退 powershell（PS5.1）
    if Command::new("pwsh").arg("--version").output().is_ok() {
        return Ok("pwsh");
    }
    Ok("powershell")
}

/// 仅做语法校验（不提取元数据），返回 Ok(()) 或错误列表。
pub fn validate_syntax(content: &str) -> DstResult<()> {
    let parsed = parse_ps1(content)?;
    if !parsed.parse_errors.is_empty() {
        return Err(DstError::PsParse(parsed.parse_errors.join("; ")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_files_ps1() {
        let code = include_str!("../../templates/Public/Files.ps1");
        let r = parse_ps1(code).expect("parse");
        let cat = r.category.expect("category");
        assert_eq!(cat.name, "files");
        assert_eq!(cat.title, "文件管理");
        assert!(cat.aliases.contains(&"文件".to_string()));
        // Files.ps1 应含 lt/ltf/ltd/ll/la/mkcd/up/up2/open/here/sz
        let names: Vec<_> = r.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"lt"));
        assert!(names.contains(&"sz"));
        assert!(names.contains(&"mkcd"));
        // 每个函数应有 synopsis
        for f in &r.functions {
            assert!(!f.synopsis.is_empty(), "函数 {} 缺 synopsis", f.name);
        }
    }

    #[test]
    fn parse_git_ps1() {
        let code = include_str!("../../templates/Public/Git.ps1");
        let r = parse_ps1(code).expect("parse");
        let cat = r.category.expect("category");
        assert_eq!(cat.name, "git");
        let names: Vec<_> = r.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"gs"));
        assert!(names.contains(&"gg"));
        assert!(names.contains(&"gclean"));
    }

    #[test]
    fn parse_help_ps1_has_dsh() {
        let code = include_str!("../../templates/Public/Help.ps1");
        let r = parse_ps1(code).expect("parse");
        // Help.ps1 无 @DST-Category 块（它是公共部分）
        assert!(r.category.is_none());
        let names: Vec<_> = r.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"dsh"));
        assert!(names.contains(&"Show-DstCategories"));
    }

    #[test]
    fn parse_invalid_syntax_fails() {
        let bad = "function { invalid syntax here";
        let r = parse_ps1(bad);
        assert!(r.is_err());
    }
}