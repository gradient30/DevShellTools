use crate::error::{DstError, DstResult};
use std::path::PathBuf;

/// 工作区固定路径：Documents\DevShellTools
pub fn workspace_root() -> PathBuf {
    let docs = std::env::var("USERPROFILE")
        .map(|p| PathBuf::from(p).join("Documents"))
        .unwrap_or_else(|_| PathBuf::from("."));
    docs.join("DevShellTools")
}

/// .studio 子目录（日志、运行时元数据）
pub fn studio_dir() -> PathBuf {
    workspace_root().join(".studio")
}

/// 工作区元数据文件
pub fn meta_file() -> PathBuf {
    studio_dir().join("workspace.json")
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceMeta {
    pub version: String,
    pub template_version: String,
    pub created_at: String,
    pub last_sync: String,
}

/// 工作区是否已初始化（核心文件齐全）。
pub fn is_initialized() -> bool {
    let root = workspace_root();
    root.join("DevShellTools.psd1").exists()
        && root.join("DevShellTools.psm1").exists()
        && root.join("Private").join("Common.ps1").exists()
        && root.join("Public").exists()
        && meta_file().exists()
}

/// 校验工作区必需文件存在，返回缺失项列表。
pub fn missing_files() -> Vec<String> {
    let root = workspace_root();
    let required = [
        "DevShellTools.psd1",
        "DevShellTools.psm1",
        "install.ps1",
        "uninstall.ps1",
        "Private/Common.ps1",
    ];
    let mut missing = vec![];
    for r in required {
        if !root.join(r).exists() {
            missing.push(r.to_string());
        }
    }
    if !root.join("Public").exists() {
        missing.push("Public/".into());
    }
    missing
}

/// 从模板初始化工作区。首次启动调用。
pub fn init_from_template() -> DstResult<()> {
    let root = workspace_root();
    if root.exists() && is_initialized() {
        return Err(DstError::WorkspaceExists(root.display().to_string()));
    }
    std::fs::create_dir_all(&root)?;
    crate::template::write_template_to(&root)?;
    std::fs::create_dir_all(studio_dir())?;

    let now = chrono::Utc::now().to_rfc3339();
    let meta = WorkspaceMeta {
        version: crate::template::TEMPLATE_VERSION.to_string(),
        template_version: crate::template::TEMPLATE_VERSION.to_string(),
        created_at: now.clone(),
        last_sync: now,
    };
    let json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(meta_file(), json)?;
    Ok(())
}

/// 更新 last_sync 时间戳。
pub fn touch_last_sync() -> DstResult<()> {
    if !meta_file().exists() {
        return Ok(());
    }
    let raw = std::fs::read_to_string(meta_file())?;
    let mut meta: WorkspaceMeta = serde_json::from_str(&raw)?;
    meta.last_sync = chrono::Utc::now().to_rfc3339();
    std::fs::write(meta_file(), serde_json::to_string_pretty(&meta)?)?;
    Ok(())
}

/// 列出 Public 目录下所有 .ps1 文件名（含扩展名）。
pub fn list_public_files() -> DstResult<Vec<String>> {
    let public = workspace_root().join("Public");
    if !public.exists() {
        return Err(DstError::WorkspaceBroken("Public 目录不存在".into()));
    }
    let mut names = vec![];
    for e in std::fs::read_dir(&public)? {
        let e = e?;
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) == Some("ps1") {
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    Ok(names)
}

/// 读取工作区某文件全文。
pub fn read_file(rel: &str) -> DstResult<String> {
    let path = workspace_root().join(rel);
    if !path.exists() {
        return Err(DstError::FileNotFound(rel.to_string()));
    }
    Ok(std::fs::read_to_string(path)?)
}

/// 写工作区某文件全文（不 git 提交，调用方负责快照）。
pub fn write_file(rel: &str, content: &str) -> DstResult<()> {
    let path = workspace_root().join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, content)?;
    Ok(())
}

/// 删除工作区某文件（不 git 提交）。
pub fn delete_file(rel: &str) -> DstResult<()> {
    let path = workspace_root().join(rel);
    if !path.exists() {
        return Ok(());
    }
    std::fs::remove_file(path)?;
    Ok(())
}

/// 工作区状态摘要（给前端展示）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkspaceStatus {
    pub initialized: bool,
    pub root: String,
    pub version: String,
    pub template_version: String,
    pub created_at: String,
    pub last_sync: String,
    pub missing_files: Vec<String>,
    pub public_files: Vec<String>,
}

pub fn status() -> DstResult<WorkspaceStatus> {
    let root = workspace_root();
    let initialized = is_initialized();
    let (version, template_version, created_at, last_sync) = if meta_file().exists() {
        match std::fs::read_to_string(meta_file()) {
            Ok(raw) => match serde_json::from_str::<WorkspaceMeta>(&raw) {
                Ok(m) => (m.version, m.template_version, m.created_at, m.last_sync),
                Err(_) => ("unknown".into(), "unknown".into(), "".into(), "".into()),
            },
            Err(_) => ("unknown".into(), "unknown".into(), "".into(), "".into()),
        }
    } else {
        (
            crate::template::TEMPLATE_VERSION.into(),
            crate::template::TEMPLATE_VERSION.into(),
            "".into(),
            "".into(),
        )
    };
    let missing_files = if initialized {
        vec![]
    } else {
        missing_files()
    };
    let public_files = if initialized {
        list_public_files().unwrap_or_default()
    } else {
        vec![]
    };
    Ok(WorkspaceStatus {
        initialized,
        root: root.display().to_string(),
        version,
        template_version,
        created_at,
        last_sync,
        missing_files,
        public_files,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_path_is_documents_subdir() {
        let root = workspace_root();
        assert!(root.to_string_lossy().ends_with("DevShellTools"));
    }
}