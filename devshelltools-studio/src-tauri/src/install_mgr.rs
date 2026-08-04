use crate::error::{DstError, DstResult};
use crate::workspace;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallStatus {
    pub workspace_ready: bool,
    pub ps51_module_present: bool,
    pub ps7_module_present: bool,
    pub profile_configured: bool,
    /// profile 已配置且 PS7 副本存在时视为已安装
    pub installed: bool,
}

fn documents_dir() -> PathBuf {
    std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Documents"))
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn ps51_module_dir() -> PathBuf {
    documents_dir()
        .join("WindowsPowerShell")
        .join("Modules")
        .join("DevShellTools")
}

pub fn ps7_module_dir() -> PathBuf {
    documents_dir()
        .join("PowerShell")
        .join("Modules")
        .join("DevShellTools")
}

fn profile_paths() -> Vec<PathBuf> {
    let docs = documents_dir();
    vec![
        docs.join("WindowsPowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
        docs.join("PowerShell")
            .join("Microsoft.PowerShell_profile.ps1"),
    ]
}

fn profile_has_import(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    std::fs::read_to_string(path)
        .map(|c| {
            c.lines()
                .any(|l| l.trim().starts_with("Import-Module DevShellTools"))
        })
        .unwrap_or(false)
}

pub fn install_status() -> InstallStatus {
    let workspace_ready = workspace::is_initialized();
    let ps51 = ps51_module_dir();
    let ps7 = ps7_module_dir();
    let ps51_module_present = ps51.join("DevShellTools.psd1").exists();
    let ps7_module_present = ps7.join("DevShellTools.psd1").exists();
    let profile_configured = profile_paths().iter().any(|p| profile_has_import(p));
    let installed = workspace_ready && profile_configured && ps7_module_present;
    InstallStatus {
        workspace_ready,
        ps51_module_present,
        ps7_module_present,
        profile_configured,
        installed,
    }
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let name = entry.file_name();
        if name == ".git" || name == ".studio" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(name);
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn ensure_profile_import(path: &Path) -> DstResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if !path.exists() {
        std::fs::write(path, "")?;
    }
    if profile_has_import(path) {
        return Ok(());
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines = content;
    if !lines.ends_with('\n') && !lines.is_empty() {
        lines.push('\n');
    }
    lines.push_str("\n# DevShellTools\nImport-Module DevShellTools -Force -ErrorAction SilentlyContinue\n");
    std::fs::write(path, lines)?;
    Ok(())
}

fn remove_profile_import(path: &Path) -> DstResult<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    let filtered: Vec<&str> = content
        .lines()
        .filter(|l| {
            !l.trim().eq("# DevShellTools")
                && !l.trim().starts_with("Import-Module DevShellTools")
        })
        .collect();
    std::fs::write(path, filtered.join("\n"))?;
    Ok(())
}

/// 软安装：同步 PS7 模块目录 + 写入 Profile。
pub fn install_module() -> DstResult<InstallStatus> {
    if !workspace::is_initialized() {
        return Err(DstError::Other("请先初始化工作区".into()));
    }
    let src = workspace::workspace_root();
    let ps7 = ps7_module_dir();
    if ps7.exists() {
        std::fs::remove_dir_all(&ps7).ok();
    }
    copy_dir_all(&src, &ps7).map_err(|e| DstError::Other(format!("复制到 PS7 模块目录失败：{e}")))?;

    for p in profile_paths() {
        ensure_profile_import(&p)?;
    }
    Ok(install_status())
}

/// 软卸载：仅清理 Profile + 删除 PS7 副本，保留 PS5.1 工作区。
pub fn uninstall_module() -> DstResult<InstallStatus> {
    for p in profile_paths() {
        remove_profile_import(&p)?;
    }
    let ps7 = ps7_module_dir();
    if ps7.exists() {
        std::fs::remove_dir_all(&ps7).ok();
    }
    Ok(install_status())
}
