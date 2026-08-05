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

/// 安装：直接执行工作区的 install.ps1，秒速完成。
/// 如果工作区没有 install.ps1，用内嵌模板的 install.ps1 写临时文件执行。
pub fn install_module() -> DstResult<InstallResult> {
    let ws = workspace::workspace_root();
    let install_ps1 = ws.join("install.ps1");

    // 工作区无 install.ps1 → 用内嵌模板写临时文件
    let (script_path, is_temp) = if install_ps1.exists() {
        (install_ps1, false)
    } else {
        let tmp = std::env::temp_dir().join("dst-install.ps1");
        std::fs::write(&tmp, crate::template::INSTALL_PS1)
            .map_err(|e| DstError::Other(format!("写临时 install.ps1 失败：{e}")))?;
        (tmp, true)
    };

    let exe = "powershell";
    let mut cmd = Command::new(exe);
    for arg in ps_base_args(exe) { cmd.arg(arg); }
    cmd.arg("-File").arg(&script_path);
    let output = output_hidden(cmd)
        .map_err(|e| DstError::Other(format!("启动 install.ps1 失败：{e}")))?;

    if is_temp { std::fs::remove_file(&script_path).ok(); }

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

/// 卸载：直接执行工作区的 uninstall.ps1，无则用内嵌模板。
pub fn uninstall_module() -> DstResult<InstallResult> {
    let ws = workspace::workspace_root();
    let uninstall_ps1 = ws.join("uninstall.ps1");
    let (script_path, is_temp) = if uninstall_ps1.exists() {
        (uninstall_ps1, false)
    } else {
        let tmp = std::env::temp_dir().join("dst-uninstall.ps1");
        std::fs::write(&tmp, crate::template::UNINSTALL_PS1)
            .map_err(|e| DstError::Other(format!("写临时 uninstall.ps1 失败：{e}")))?;
        (tmp, true)
    };
    let exe = "powershell";
    let mut cmd = Command::new(exe);
    for arg in ps_base_args(exe) { cmd.arg(arg); }
    cmd.arg("-File").arg(&script_path);
    let output = output_hidden(cmd)
        .map_err(|e| DstError::Other(format!("启动 uninstall.ps1 失败：{e}")))?;
    if is_temp { std::fs::remove_file(&script_path).ok(); }
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DstError::Other(format!("uninstall.ps1 执行失败：{}", stderr.trim())));
    }
    let status = install_status();
    let verified = !status.installed;
    Ok(InstallResult { status, message: "卸载完成。新开 PowerShell 窗口生效。".into(), verified })
}

// ============ 回退方案 ============

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == ".git" || name == ".studio" || name == "install.ps1" || name == "uninstall.ps1" || name == "README.md" { continue; }
        let from = entry.path();
        let to = dst.join(name);
        if from.is_dir() { copy_dir_all(&from, &to)?; } else { std::fs::copy(&from, &to)?; }
    }
    Ok(())
}

fn ensure_profile_import(path: &Path) -> DstResult<()> {
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent)?; }
    if !path.exists() { std::fs::write(path, "")?; }
    if profile_has_import(path) { return Ok(()); }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines = content;
    if !lines.ends_with('\n') && !lines.is_empty() { lines.push('\n'); }
    lines.push_str("\n# DevShellTools\nImport-Module DevShellTools -Force -ErrorAction SilentlyContinue\n");
    std::fs::write(path, lines)?;
    Ok(())
}

fn install_via_copy() -> DstResult<InstallResult> {
    let src = workspace::workspace_root();
    for target in [ps51_module_dir(), ps7_module_dir()] {
        if target.exists() && target != src { std::fs::remove_dir_all(&target).ok(); }
        copy_dir_all(&src, &target).map_err(|e| DstError::Other(format!("复制失败：{e}")))?;
    }
    for p in profile_paths() { ensure_profile_import(&p)?; }
    let status = install_status();
    let verified = status.installed;
    Ok(InstallResult { status, message: "安装完成。".into(), verified })
}

fn uninstall_via_remove() -> DstResult<InstallResult> {
    for p in profile_paths() {
        if p.exists() {
            let content = std::fs::read_to_string(&p).unwrap_or_default();
            let filtered: Vec<&str> = content.lines()
                .filter(|l| !l.trim().eq("# DevShellTools") && !l.trim().starts_with("Import-Module DevShellTools"))
                .collect();
            std::fs::write(&p, filtered.join("\n"))?;
        }
    }
    let src = workspace::workspace_root();
    for target in [ps51_module_dir(), ps7_module_dir()] {
        if target.exists() && target != src { std::fs::remove_dir_all(&target).ok(); }
    }
    let status = install_status();
    let verified = !status.installed;
    Ok(InstallResult { status, message: "卸载完成。".into(), verified })
}