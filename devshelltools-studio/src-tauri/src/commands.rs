use crate::consistency;
use crate::error::{DstError, DstResult};
use crate::git;
use crate::ps_parser;
use crate::safety;
use crate::sync::{self, CategoryInfo};
use crate::workspace;

fn root() -> std::path::PathBuf {
    workspace::workspace_root()
}

// ============ 工作区管理 ============

#[tauri::command]
pub fn workspace_status() -> DstResult<workspace::WorkspaceStatus> {
    workspace::status()
}

#[tauri::command]
pub fn init_workspace() -> DstResult<String> {
    if workspace::is_initialized() {
        return Err(DstError::WorkspaceExists(
            workspace::workspace_root().display().to_string(),
        ));
    }
    workspace::init_from_template()?;
    let r = root();
    git::init_repo(&r)?;
    workspace::touch_last_sync()?;
    git::head_oid(&r)
}

// ============ 读取 ============

#[tauri::command]
pub fn list_public_files() -> DstResult<Vec<String>> {
    workspace::list_public_files()
}

#[tauri::command]
pub fn read_workspace_file(rel: String) -> DstResult<String> {
    workspace::read_file(&rel)
}

/// 列出所有分类及其函数（前端展示用）。
#[tauri::command]
pub fn list_categories() -> DstResult<Vec<CategoryInfo>> {
    sync::scan_categories()
}

/// 读取某分类文件的完整内容。
#[tauri::command]
pub fn read_category_file(file_name: String) -> DstResult<String> {
    let rel = format!("Public/{file_name}");
    workspace::read_file(&rel)
}

// ============ 写入（带 git 快照 + 安全检查 + 重生成）============

/// 写工作区文件，并自动 git 快照。
#[tauri::command]
pub fn write_workspace_file(rel: String, content: String, message: String) -> DstResult<String> {
    workspace::write_file(&rel, &content)?;
    let r = root();
    let oid = git::snapshot(&r, &message)?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

/// 删除工作区文件，并自动 git 快照。
#[tauri::command]
pub fn delete_workspace_file(rel: String, message: String) -> DstResult<String> {
    workspace::delete_file(&rel)?;
    let r = root();
    let oid = git::snapshot(&r, &message)?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

/// 创建新分类文件。file_name 如 "Docker.ps1"，content 含 @DST-Category 块 + 函数。
/// 写入后自动重生成公共部分 + git 快照。
#[tauri::command]
pub fn create_category(
    file_name: String,
    content: String,
    message: String,
) -> DstResult<String> {
    if !file_name.ends_with(".ps1") {
        return Err(DstError::Other("分类文件名必须以 .ps1 结尾".into()));
    }
    let rel = format!("Public/{file_name}");
    if workspace::read_file(&rel).is_ok() {
        return Err(DstError::Other(format!("分类文件已存在：{file_name}")));
    }
    // 安全检查
    let report = safety::check(&content)?;
    if !report.ok {
        return Err(DstError::SafetyBlocked(report.violations.join("; ")));
    }
    // 语法校验
    ps_parser::validate_syntax(&content)?;
    workspace::write_file(&rel, &content)?;
    sync::regenerate_all()?;
    let r = root();
    let oid = git::snapshot(&r, &format!("新建分类：{message}"))?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

/// 删除分类文件，自动重生成 + 快照。
#[tauri::command]
pub fn delete_category(file_name: String, message: String) -> DstResult<String> {
    let rel = format!("Public/{file_name}");
    workspace::delete_file(&rel)?;
    sync::regenerate_all()?;
    let r = root();
    let oid = git::snapshot(&r, &format!("删除分类：{message}"))?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

/// 更新分类文件内容（覆盖写入），自动安全检查 + 语法校验 + 重生成 + 快照。
#[tauri::command]
pub fn update_category_file(
    file_name: String,
    content: String,
    message: String,
) -> DstResult<String> {
    let rel = format!("Public/{file_name}");
    let report = safety::check(&content)?;
    if !report.ok {
        return Err(DstError::SafetyBlocked(report.violations.join("; ")));
    }
    ps_parser::validate_syntax(&content)?;
    workspace::write_file(&rel, &content)?;
    sync::regenerate_all()?;
    let r = root();
    let oid = git::snapshot(&r, &format!("更新分类：{message}"))?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

/// 手动触发公共部分重生成 + 快照。
#[tauri::command]
pub fn sync_public(message: String) -> DstResult<String> {
    sync::regenerate_all()?;
    let r = root();
    let oid = git::snapshot(&r, &message)?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

// ============ 校验 ============

/// 三方一致性校验。
#[tauri::command]
pub fn consistency_check() -> DstResult<consistency::ConsistencyReport> {
    consistency::check()
}

/// 安全规则检查（不写盘，仅返回结果）。
#[tauri::command]
pub fn safety_check(code: String) -> DstResult<safety::SafetyReport> {
    safety::check(&code)
}

/// PS 语法校验（不写盘）。
#[tauri::command]
pub fn validate_ps_syntax(code: String) -> DstResult<()> {
    ps_parser::validate_syntax(&code)
}

// ============ Git ============

#[tauri::command]
pub fn git_log(n: Option<usize>) -> DstResult<Vec<git::CommitInfo>> {
    let r = root();
    git::log(&r, n.unwrap_or(20))
}

#[tauri::command]
pub fn git_reset_hard(oid: String) -> DstResult<()> {
    let r = root();
    git::reset_hard(&r, &oid)
}

#[tauri::command]
pub fn git_snapshot(message: String) -> DstResult<String> {
    let r = root();
    let oid = git::snapshot(&r, &message)?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_returns_workspace_root() {
        let r = root();
        assert!(r.to_string_lossy().ends_with("DevShellTools"));
    }
}