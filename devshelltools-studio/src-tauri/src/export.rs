use crate::error::{DstError, DstResult};
use crate::workspace;
use std::path::Path;

/// 导出工作区到目标目录（完整复制，不含 .git）。
/// target_dir 由前端通过 dialog 选择。
pub fn export_to(target_dir: &str) -> DstResult<String> {
    let target = Path::new(target_dir);
    if !target.exists() {
        std::fs::create_dir_all(target)?;
    }
    let ws = workspace::workspace_root();
    if !ws.exists() {
        return Err(DstError::WorkspaceNotFound(ws.display().to_string()));
    }
    copy_dir_exclude_git(&ws, target)?;
    Ok(target.display().to_string())
}

/// 从源目录导入工作区（覆盖当前工作区）。
/// source_dir 由前端通过 dialog 选择。
pub fn import_from(source_dir: &str) -> DstResult<Vec<String>> {
    let source = Path::new(source_dir);
    if !source.exists() {
        return Err(DstError::FileNotFound(source_dir.into()));
    }
    // 校验源目录是有效工作区（含 DevShellTools.psd1）
    if !source.join("DevShellTools.psd1").exists() {
        return Err(DstError::WorkspaceBroken("源目录缺少 DevShellTools.psd1".into()));
    }
    let ws = workspace::workspace_root();
    // 确保工作区父目录存在
    if let Some(parent) = ws.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // 清理当前工作区（保留 .git）
    if ws.exists() {
        // 保留 .git 目录
        let git_backup = ws.join(".git");
        let git_tmp = ws.parent().unwrap().join(".dst-git-backup");
        if git_backup.exists() {
            std::fs::rename(&git_backup, &git_tmp)?;
        }
        std::fs::remove_dir_all(&ws)?;
        std::fs::create_dir_all(&ws)?;
        if git_tmp.exists() {
            std::fs::rename(&git_tmp, &git_backup)?;
        }
    }
    // 复制源目录到工作区（不含 .git）
    let mut imported = vec![];
    copy_dir_exclude_git(source, &ws)?;
    // 记录导入的文件
    for entry in std::fs::read_dir(&ws)? {
        let e = entry?;
        if let Some(name) = e.file_name().to_str() {
            imported.push(name.to_string());
        }
    }
    // 重生成 + 快照
    crate::sync::regenerate_all()?;
    let oid = crate::git::snapshot(&ws, "import: 导入工作区备份")?;
    workspace::touch_last_sync()?;
    imported.push(format!("git commit: {oid}"));
    Ok(imported)
}

fn copy_dir_exclude_git(src: &Path, dst: &Path) -> DstResult<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // 跳过 .git 和 .studio（运行时元数据）
        if name_str == ".git" || name_str == ".studio" {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        if src_path.is_dir() {
            copy_dir_exclude_git(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn tmp_dir() -> std::path::PathBuf {
        let mut p = env::temp_dir();
        p.push(format!(
            "dst-export-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn copy_dir_excludes_git_and_studio() {
        let src = tmp_dir();
        std::fs::create_dir_all(src.join(".git")).unwrap();
        std::fs::create_dir_all(src.join(".studio")).unwrap();
        std::fs::create_dir_all(src.join("Public")).unwrap();
        std::fs::write(src.join("DevShellTools.psd1"), "test").unwrap();
        std::fs::write(src.join("Public").join("Test.ps1"), "function t {}").unwrap();

        let dst = tmp_dir();
        copy_dir_exclude_git(&src, &dst).unwrap();

        assert!(dst.join("DevShellTools.psd1").exists());
        assert!(dst.join("Public").join("Test.ps1").exists());
        assert!(!dst.join(".git").exists());
        assert!(!dst.join(".studio").exists());

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dst);
    }
}