use crate::error::{DstError, DstResult};
use crate::process_util::{output_hidden, ps_base_args};
use crate::workspace;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallStatus {
    pub workspace_ready: bool,
    pub ps51_module_present: bool,
    pub ps7_module_present: bool,
    pub profile_configured: bool,
    /// profile 已配置且 PS7 副本存在时视为已安装
    pub installed: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallResult {
    pub status: InstallStatus,
    /// 给用户的即时说明（含是否需新开 PowerShell）
    pub message: String,
    /// 隐藏子进程中验证 Import-Module / dsh 是否可用
    pub verified: bool,
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

/// 在独立进程中验证模块是否可加载（不依赖当前终端会话）。
fn verify_module_load() -> bool {
    let script = r#"
$ErrorActionPreference = 'Stop'
Import-Module DevShellTools -Force -ErrorAction Stop
if (-not (Get-Command dsh -ErrorAction SilentlyContinue)) { throw 'dsh missing' }
'OK'
"#;
    let exe = if Command::new("pwsh").arg("--version").output().is_ok() {
        "pwsh"
    } else {
        "powershell"
    };
    let mut cmd = Command::new(exe);
    for arg in ps_base_args(exe) {
        cmd.arg(arg);
    }
    cmd.args(["-Command", script]);
    output_hidden(cmd)
        .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).contains("OK"))
        .unwrap_or(false)
}

fn install_result(status: InstallStatus, action: &str, verified: bool) -> InstallResult {
    let message = if action == "install" {
        if status.installed && verified {
            "安装完成：Profile 与 PS7 模块已更新，验证通过。请新开 PowerShell 窗口运行 dsh。".into()
        } else if status.installed {
            "安装完成：Profile 与 PS7 模块已更新。请新开 PowerShell 窗口运行 dsh（自动验证未通过，可手动 Import-Module DevShellTools）。".into()
        } else {
            "安装未完成：请检查 Profile 权限或 PS7 模块目录。".into()
        }
    } else if status.installed {
        "卸载未完成：仍有残留配置。".into()
    } else if verified {
        "卸载完成：已移除 Profile 导入与 PS7 副本。已打开的 PowerShell 窗口需重启后生效。".into()
    } else {
        "卸载完成：已移除 Profile 导入与 PS7 副本。请新开 PowerShell 窗口确认 dsh 不可用。".into()
    };
    InstallResult {
        status,
        message,
        verified,
    }
}

/// 软安装：同步 PS7 模块目录 + 写入 Profile。
pub fn install_module() -> DstResult<InstallResult> {
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
    let status = install_status();
    let verified = verify_module_load();
    Ok(install_result(status, "install", verified))
}

/// 软卸载：仅清理 Profile + 删除 PS7 副本，保留 PS5.1 工作区。
pub fn uninstall_module() -> DstResult<InstallResult> {
    for p in profile_paths() {
        remove_profile_import(&p)?;
    }
    let ps7 = ps7_module_dir();
    if ps7.exists() {
        std::fs::remove_dir_all(&ps7).ok();
    }
    let status = install_status();
    let verified = verify_module_uninstalled(&status);
    Ok(install_result(status, "uninstall", verified))
}

fn verify_module_uninstalled(status: &InstallStatus) -> bool {
    !status.profile_configured && !status.ps7_module_present
}
