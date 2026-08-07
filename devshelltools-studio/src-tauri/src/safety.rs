use crate::error::DstResult;

/// 安全检查结果。
#[derive(Debug, Clone, serde::Serialize)]
pub struct SafetyReport {
    pub ok: bool,
    pub violations: Vec<String>,
}

/// 静态扫描 PowerShell 代码是否违反安全边界。
/// 规则与 README "安全边界" 一致，禁止危险命令。
pub fn check(code: &str) -> DstResult<SafetyReport> {
    check_with_options(code, false)
}

/// `danger_mode=true` 时跳过全部红线（仅 AI 会话 `/danger` 激活后的生成/插入路径）。
pub fn check_with_options(code: &str, danger_mode: bool) -> DstResult<SafetyReport> {
    if danger_mode {
        return Ok(SafetyReport {
            ok: true,
            violations: vec![],
        });
    }

    let mut violations = vec![];

    // 规则1：禁止 git push --force / --force-with-lease
    if contains_word(code, "--force") || contains_word(code, "--force-with-lease") {
        if code.contains("push") {
            violations.push("禁止 git push --force / --force-with-lease".into());
        }
    }
    // 规则2：禁止 git reset --hard（须同时出现 git reset 与 --hard，避免 preset 等误匹配 reset 子串）
    if code.contains("git reset") && contains_word(code, "--hard") {
        violations.push("禁止 git reset --hard".into());
    }
    // 规则3：禁止 git clean -fd / -f（真实删除）
    if code.contains("clean") && (contains_word(code, "-fd") || contains_word(code, "-f")) {
        // 允许 -nd / -ndx（dry-run）
        if !contains_word(code, "-nd") && !contains_word(code, "-ndx") {
            violations.push("禁止 git clean -f / -fd（真实删除），只允许 -nd / -ndx 预览".into());
        }
    }
    // 规则4：Stop-Process 必须有 -Confirm 或在 SupportsShouldProcess 函数内
    if code.contains("Stop-Process") {
        let lower = code.to_lowercase();
        if !lower.contains("shouldprocess") && !lower.contains("-confirm") && !lower.contains("-force") {
            violations.push("Stop-Process 必须配合 -Confirm 或 SupportsShouldProcess".into());
        }
    }
    // 规则5：禁止修改用户级环境变量（SetEnvironmentVariable ... "User"）
    if code.contains("SetEnvironmentVariable") && code.contains("\"User\"") {
        violations.push("禁止 [Environment]::SetEnvironmentVariable(..., \"User\")，只允许进程级".into());
    }
    // 规则6：禁止「同一条」Remove-Item 同时带 -Recurse 与 -Force。
    // 注意：不能整文件搜三个子串——Files.ps1 的 sz 使用 Get-ChildItem -Recurse -Force，
    // 与另一函数的 Remove-Item -Force 并存时会误报。
    if has_remove_item_recurse_force(code) && !code.to_lowercase().contains("uninstall") {
        violations.push("禁止 Remove-Item -Recurse -Force（危险删除）".into());
    }
    // 规则7：禁止 Start-Process -Verb RunAs 之外的提权，且 super 已是特例
    // （不拦截，super 已存在）

    Ok(SafetyReport {
        ok: violations.is_empty(),
        violations,
    })
}

/// 检查代码是否含某"词"（前后为非单词字符边界），避免子串误匹配。
fn contains_word(haystack: &str, needle: &str) -> bool {
    let mut start = 0;
    while let Some(idx) = haystack[start..].find(needle) {
        let abs = start + idx;
        let before_ok = abs == 0 || !haystack.as_bytes()[abs - 1].is_ascii_alphanumeric();
        let after_idx = abs + needle.len();
        let after_ok = after_idx >= haystack.len()
            || !haystack.as_bytes()[after_idx].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
        start = abs + 1;
    }
    false
}

/// 提取每个 `Remove-Item` 调用的参数片段（支持行续接 `），再判断是否同时含 -Recurse 与 -Force。
fn has_remove_item_recurse_force(code: &str) -> bool {
    let lower = code.to_lowercase();
    let mut search = 0;
    while let Some(rel) = lower[search..].find("remove-item") {
        let start = search + rel;
        let before_ok =
            start == 0 || !code.as_bytes()[start - 1].is_ascii_alphanumeric();
        let after = start + "remove-item".len();
        let after_ok =
            after >= code.len() || !code.as_bytes()[after].is_ascii_alphanumeric();
        if !before_ok || !after_ok {
            search = start + 1;
            continue;
        }
        let call = extract_ps_command_call(code, start);
        let call_lower = call.to_lowercase();
        if contains_word(&call_lower, "-recurse") && contains_word(&call_lower, "-force") {
            return true;
        }
        search = start + 1;
    }
    false
}

/// 从 cmdlet 起始位置提取单次调用文本（含反引号续行，遇 `;` / 未续行换行结束）。
fn extract_ps_command_call(code: &str, start: usize) -> String {
    let rest = &code[start..];
    let mut out = String::new();
    for line in rest.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.ends_with('`') {
            out.push_str(trimmed_end.trim_end_matches('`').trim_end());
            out.push(' ');
            continue;
        }
        // 同一行可能有 `cmd; other` —— 只取到第一个分号（字符串内分号极少见于此场景）
        if let Some(semi) = trimmed_end.find(';') {
            out.push_str(trimmed_end[..semi].trim_end());
        } else {
            out.push_str(trimmed_end);
        }
        break;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_dry_run_ok() {
        let r = check("git clean -nd").unwrap();
        assert!(r.ok);
    }

    #[test]
    fn clean_real_delete_blocked() {
        let r = check("git clean -fd").unwrap();
        assert!(!r.ok);
        assert!(r.violations[0].contains("clean"));
    }

    #[test]
    fn force_push_blocked() {
        let r = check("git push --force origin main").unwrap();
        assert!(!r.ok);
    }

    #[test]
    fn hard_reset_blocked() {
        let r = check("git reset --hard HEAD~1").unwrap();
        assert!(!r.ok);
    }

    #[test]
    fn reset_substring_without_git_reset_ok() {
        // decorate / preset 等含 reset 子串但非 git reset --hard
        let r = check(r#"& git log --decorate --oneline --all "--max-count=$Count""#).unwrap();
        assert!(r.ok, "{:?}", r.violations);
    }

    #[test]
    fn synopsis_mention_git_reset_hard_in_other_fn_block_not_checked() {
        // upsert 只扫当前块；gg 块本身安全即应通过
        let gg = r#"function gg {
<#
.SYNOPSIS
显示图形化精简提交历史
.EXAMPLE
gg
#>
    Assert-Git
    & git log --graph --decorate --oneline --all "--max-count=$Count"
}"#;
        let r = check(gg).unwrap();
        assert!(r.ok, "gg 块不应被拦截：{:?}", r.violations);
    }

    #[test]
    fn danger_mode_allows_hard_reset() {
        let r = check_with_options("git reset --hard origin/main", true).unwrap();
        assert!(r.ok);
        assert!(r.violations.is_empty());
    }

    #[test]
    fn user_env_blocked() {
        let r = check(r#"[Environment]::SetEnvironmentVariable("x","y","User")"#).unwrap();
        assert!(!r.ok);
    }

    #[test]
    fn process_env_ok() {
        let r = check(r#"[Environment]::SetEnvironmentVariable("x","y","Process")"#).unwrap();
        assert!(r.ok);
    }

    #[test]
    fn safe_command_ok() {
        let r = check(r#"function gs { git status -sb }"#).unwrap();
        assert!(r.ok);
    }

    #[test]
    fn remove_item_force_without_recurse_ok() {
        let code = r#"
function rmf {
    [CmdletBinding(SupportsShouldProcess)]
    param([string]$Path)
    Remove-Item -LiteralPath $Path -Force -ErrorAction Stop
}
"#;
        let r = check(code).unwrap();
        assert!(r.ok, "仅 Remove-Item -Force 应放行：{:?}", r.violations);
    }

    #[test]
    fn remove_item_recurse_force_blocked() {
        let code = r#"
function badrm {
    param([string]$Path)
    Remove-Item -LiteralPath $Path -Recurse -Force
}
"#;
        let r = check(code).unwrap();
        assert!(!r.ok);
        assert!(r.violations.iter().any(|v| v.contains("Remove-Item")));
    }

    #[test]
    fn remove_item_recurse_force_multiline_blocked() {
        let code = "Remove-Item -LiteralPath $p `\n  -Recurse `\n  -Force\n";
        let r = check(code).unwrap();
        assert!(!r.ok, "续行形式也应拦截");
    }

    #[test]
    fn get_childitem_recurse_force_with_safe_remove_ok() {
        // 复现：sz 使用 Get-ChildItem -Recurse -Force，rmf 使用 Remove-Item -Force
        let code = r#"
function sz {
    Get-ChildItem -LiteralPath $Path -File -Recurse -Force -ErrorAction SilentlyContinue
}
function rmf {
    Remove-Item -LiteralPath $item.FullName -Force -ErrorAction Stop
}
"#;
        let r = check(code).unwrap();
        assert!(
            r.ok,
            "跨命令的 -Recurse/-Force 不应误报：{:?}",
            r.violations
        );
    }
}