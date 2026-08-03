use crate::error::{DstError, DstResult};
use crate::workspace;
use std::path::PathBuf;

/// 旧版 install.ps1 安装的两个目标目录
fn legacy_install_dirs() -> Vec<PathBuf> {
    let docs = std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Documents"))
        .unwrap_or_else(|_| PathBuf::from("."));
    vec![
        docs.join("WindowsPowerShell").join("Modules").join("DevShellTools"),
        docs.join("PowerShell").join("Modules").join("DevShellTools"),
    ]
}

/// 检测旧版安装（install.ps1 装的副本是否存在）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct MigrationCheck {
    pub has_legacy: bool,
    pub legacy_dirs: Vec<String>,
    pub workspace_initialized: bool,
}

pub fn check_migration() -> MigrationCheck {
    let legacy: Vec<String> = legacy_install_dirs()
        .iter()
        .filter(|d| d.exists() && d.join("DevShellTools.psd1").exists())
        .map(|d| d.display().to_string())
        .collect();
    MigrationCheck {
        has_legacy: !legacy.is_empty(),
        legacy_dirs: legacy,
        workspace_initialized: workspace::is_initialized(),
    }
}

/// 执行迁移：把旧版 Public/*.ps1 合并到便携工作区（保留用户自定义命令）。
/// 旧版的公共部分（.psd1/.psm1/Help.ps1）忽略，由 Studio 重生成。
/// 返回迁移的文件列表。
pub fn migrate_from_legacy() -> DstResult<Vec<String>> {
    let check = check_migration();
    if !check.has_legacy {
        return Err(DstError::Other("未检测到旧版安装".into()));
    }
    // 确保工作区已初始化
    if !workspace::is_initialized() {
        workspace::init_from_template()?;
        let root = workspace::workspace_root();
        crate::git::init_repo(&root)?;
    }

    let mut migrated = vec![];
    let ws_root = workspace::workspace_root();
    let ws_public = ws_root.join("Public");

    for legacy_dir in &check.legacy_dirs {
        let legacy_public = PathBuf::from(legacy_dir).join("Public");
        if !legacy_public.exists() {
            continue;
        }
        // 复制所有 .ps1 文件（覆盖工作区同名文件）
        for entry in std::fs::read_dir(&legacy_public)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("ps1") {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    let target = ws_public.join(name);
                    std::fs::copy(&path, &target)?;
                    migrated.push(name.to_string());
                }
            }
        }
    }

    // 重生成公共部分 + git 快照
    crate::sync::regenerate_all()?;
    let root = workspace::workspace_root();
    crate::git::snapshot(&root, "migrate: 从旧版 install.ps1 迁移命令")?;
    workspace::touch_last_sync()?;

    Ok(migrated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_dirs_check() {
        let dirs = legacy_install_dirs();
        assert!(dirs.iter().any(|d| d.to_string_lossy().contains("WindowsPowerShell")));
        assert!(dirs.iter().any(|d| d.to_string_lossy().contains("PowerShell")));
    }

    #[test]
    fn migration_check_returns_struct() {
        let check = check_migration();
        // 测试环境通常无旧版，但结构应正确
        assert!(check.legacy_dirs.len() <= 2);
    }
}