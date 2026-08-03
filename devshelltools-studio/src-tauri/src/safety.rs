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
    let mut violations = vec![];

    // 规则1：禁止 git push --force / --force-with-lease
    if contains_word(code, "--force") || contains_word(code, "--force-with-lease") {
        if code.contains("push") {
            violations.push("禁止 git push --force / --force-with-lease".into());
        }
    }
    // 规则2：禁止 git reset --hard
    if code.contains("reset") && contains_word(code, "--hard") {
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
    // 规则6：禁止 Remove-Item -Recurse -Force 真实删除（非 dry-run）
    if code.contains("Remove-Item") && contains_word(code, "-Recurse") && contains_word(code, "-Force") {
        // 允许在 uninstall 等明确场景，但快捷命令禁止
        if !code.contains("uninstall") {
            violations.push("禁止 Remove-Item -Recurse -Force（危险删除）".into());
        }
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
}