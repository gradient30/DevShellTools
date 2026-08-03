use crate::{error::DstResult, git, workspace};

fn root() -> std::path::PathBuf {
    workspace::workspace_root()
}

/// 前端可调：获取工作区状态摘要。
#[tauri::command]
pub fn workspace_status() -> DstResult<workspace::WorkspaceStatus> {
    workspace::status()
}

/// 前端可调：首次初始化工作区（从内嵌模板）+ git init + 首次提交。
#[tauri::command]
pub fn init_workspace() -> DstResult<String> {
    if workspace::is_initialized() {
        return Err(crate::error::DstError::WorkspaceExists(
            workspace::workspace_root().display().to_string(),
        ));
    }
    workspace::init_from_template()?;
    let r = root();
    git::init_repo(&r)?;
    workspace::touch_last_sync()?;
    git::head_oid(&r)
}

/// 前端可调：列出 Public 目录下的 .ps1 文件。
#[tauri::command]
pub fn list_public_files() -> DstResult<Vec<String>> {
    workspace::list_public_files()
}

/// 前端可调：读取工作区相对路径文件内容。
#[tauri::command]
pub fn read_workspace_file(rel: String) -> DstResult<String> {
    workspace::read_file(&rel)
}

/// 前端可调：写工作区文件，并自动 git 快照。
#[tauri::command]
pub fn write_workspace_file(rel: String, content: String, message: String) -> DstResult<String> {
    workspace::write_file(&rel, &content)?;
    let r = root();
    let oid = git::snapshot(&r, &message)?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

/// 前端可调：删除工作区文件，并自动 git 快照。
#[tauri::command]
pub fn delete_workspace_file(rel: String, message: String) -> DstResult<String> {
    workspace::delete_file(&rel)?;
    let r = root();
    let oid = git::snapshot(&r, &message)?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

/// 前端可调：最近 N 条提交。
#[tauri::command]
pub fn git_log(n: Option<usize>) -> DstResult<Vec<git::CommitInfo>> {
    let r = root();
    git::log(&r, n.unwrap_or(20))
}

/// 前端可调：回滚到某次提交。
#[tauri::command]
pub fn git_reset_hard(oid: String) -> DstResult<()> {
    let r = root();
    git::reset_hard(&r, &oid)
}

/// 前端可调：手动触发一次快照。
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