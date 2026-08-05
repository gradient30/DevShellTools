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
    pub installed: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InstallResult {
    pub status: InstallStatus,
    pub message: String,
    pub verified: bool,
}

fn documents_dir() -> PathBuf {
    crate::workspace::my_documents_path_public()
        .unwrap_or_else(|| {
            std::env::var("USERPROFILE")
                .map(|p| PathBuf::from(p).join("Documents"))
                .unwrap_or_else(|_| PathBuf::from("."))
        })
}

pub fn ps51_module_dir() -> PathBuf {
    documents_dir().join("WindowsPowerShell").join("Modules").join("DevShellTools")
}

pub fn ps7_module_dir() -> PathBuf {
    documents_dir().join("PowerShell").join("Modules").join("DevShellTools")
}

fn profile_paths() -> Vec<PathBuf> {
    let docs = documents_dir();
    vec![
        docs.join("WindowsPowerShell").join("Microsoft.PowerShell_profile.ps1"),
        docs.join("PowerShell").join("Microsoft.PowerShell_profile.ps1"),
    ]
}

fn profile_has_import(path: &Path) -> bool {
    if !path.exists() { return false; }
    std::fs::read_to_string(path)
        .map(|c| c.lines().any(|l| l.trim().starts_with("Import-Module DevShellTools")))
        .unwrap_or(false)
}

pub fn install_status() -> InstallStatus {
    let workspace_ready = workspace::workspace_root().join("DevShellTools.psd1").exists();
    let ps51_module_present = ps51_module_dir().join("DevShellTools.psd1").exists();
    let ps7_module_present = ps7_module_dir().join("DevShellTools.psd1").exists();
    let profile_configured = profile_paths().iter().any(|p| profile_has_import(p));
    let installed = workspace_ready && profile_configured && (ps51_module_present || ps7_module_present);
    InstallStatus { workspace_ready, ps51_module_present, ps7_module_present, profile_configured, installed }
}

/// 每次安装前从内嵌模板覆盖写入 install.ps1，避免工作区残留旧版自毁脚本。
fn ensure_install_script() -> DstResult<PathBuf> {
    let ws = workspace::workspace_root();
    std::fs::create_dir_all(&ws)
        .map_err(|e| DstError::Other(format!("创建工作区失败：{e}")))?;
    let install_ps1 = ws.join("install.ps1");
    std::fs::write(&install_ps1, crate::template::INSTALL_PS1)
        .map_err(|e| DstError::Other(format!("写入 install.ps1 到工作区失败：{e}")))?;
    Ok(install_ps1)
}

/// 每次卸载前从内嵌模板覆盖写入 uninstall.ps1。
fn ensure_uninstall_script() -> DstResult<PathBuf> {
    let ws = workspace::workspace_root();
    std::fs::create_dir_all(&ws)
        .map_err(|e| DstError::Other(format!("创建工作区失败：{e}")))?;
    let uninstall_ps1 = ws.join("uninstall.ps1");
    std::fs::write(&uninstall_ps1, crate::template::UNINSTALL_PS1)
        .map_err(|e| DstError::Other(format!("写入 uninstall.ps1 到工作区失败：{e}")))?;
    Ok(uninstall_ps1)
}

/// 安装：执行工作区的 install.ps1（$PSScriptRoot 指向工作区，不会误复制 temp 文件）。
pub fn install_module() -> DstResult<InstallResult> {
    let install_ps1 = ensure_install_script()?;
    let exe = "powershell";
    let mut cmd = Command::new(exe);
    for arg in ps_base_args(exe) { cmd.arg(arg); }
    cmd.arg("-File").arg(&install_ps1);
    let output = output_hidden(cmd)
        .map_err(|e| DstError::Other(format!("启动 install.ps1 失败：{e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DstError::Other(format!("install.ps1 执行失败：{}", stderr.trim())));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let status = install_status();
    let verified = status.installed;
    let msg_lines: Vec<&str> = stdout.lines()
        .filter(|l| l.contains("[成功]") || l.contains("[备份]") || l.contains("[兼容]"))
        .collect();
    let message = if msg_lines.is_empty() { "安装完成。".into() } else { format!("安装完成。\n{}", msg_lines.join("\n")) };
    Ok(InstallResult { status, message, verified })
}

/// 卸载：执行工作区的 uninstall.ps1。
pub fn uninstall_module() -> DstResult<InstallResult> {
    let uninstall_ps1 = ensure_uninstall_script()?;
    let exe = "powershell";
    let mut cmd = Command::new(exe);
    for arg in ps_base_args(exe) { cmd.arg(arg); }
    cmd.arg("-File").arg(&uninstall_ps1);
    let output = output_hidden(cmd)
        .map_err(|e| DstError::Other(format!("启动 uninstall.ps1 失败：{e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DstError::Other(format!("uninstall.ps1 执行失败：{}", stderr.trim())));
    }
    let status = install_status();
    let verified = !status.installed;
    Ok(InstallResult { status, message: "卸载完成。新开 PowerShell 窗口生效。".into(), verified })
}