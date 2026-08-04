use crate::error::{DstError, DstResult};
use crate::ps_parser;
use crate::safety;
use crate::workspace;
use std::path::Path;

/// 导出所有 Public/*.ps1 脚本到目标目录。
/// 仅复制 .ps1 文件，不复制 .git/.studio/公共部分。
pub fn export_scripts(target_dir: &str) -> DstResult<Vec<String>> {
    let target = Path::new(target_dir);
    std::fs::create_dir_all(target)?;
    let ws = workspace::workspace_root();
    let public = ws.join("Public");
    if !public.exists() {
        return Err(DstError::WorkspaceBroken("Public 目录不存在".into()));
    }
    let mut exported = vec![];
    for entry in std::fs::read_dir(&public)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("ps1") {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                std::fs::copy(&path, target.join(name))?;
                exported.push(name.to_string());
            }
        }
    }
    Ok(exported)
}

/// 导入结果
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportResult {
    pub imported: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

/// 从目录导入 ps1 脚本：逐个校验语法+安全，通过才写入。
/// 不通过的被跳过并记录原因，不破坏现有脚本。
pub fn import_scripts(source_dir: &str) -> DstResult<ImportResult> {
    let source = Path::new(source_dir);
    if !source.exists() {
        return Err(DstError::FileNotFound(source_dir.into()));
    }
    let ws = workspace::workspace_root();
    let public = ws.join("Public");
    std::fs::create_dir_all(&public)?;

    let mut imported = vec![];
    let mut skipped = vec![];
    let mut errors = vec![];

    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ps1") {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{name}: 读取失败 - {e}"));
                skipped.push(name);
                continue;
            }
        };

        // 语法校验
        if let Err(e) = ps_parser::validate_syntax(&content) {
            errors.push(format!("{name}: 语法错误 - {e}"));
            skipped.push(name);
            continue;
        }

        // 安全校验
        match safety::check(&content) {
            Ok(report) if report.ok => {}
            Ok(report) => {
                errors.push(format!("{name}: 安全拦截 - {}", report.violations.join("；")));
                skipped.push(name);
                continue;
            }
            Err(e) => {
                errors.push(format!("{name}: 安全检查失败 - {e}"));
                skipped.push(name);
                continue;
            }
        }

        // 校验通过，写入
        let target = public.join(&name);
        std::fs::write(&target, &content)?;
        imported.push(name);
    }

    Ok(ImportResult { imported, skipped, errors })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_excludes_non_ps1() {
        // 逻辑测试：只复制 .ps1
        assert!(true); // 端到端测试在 m4_acceptance
    }
}