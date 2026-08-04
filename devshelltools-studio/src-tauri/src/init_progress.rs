use crate::error::{DstError, DstResult};
use crate::{git, workspace};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct InitProgress {
    pub step: u8,
    pub label: String,
    pub percent: u8,
}

pub fn emit_progress(app: &AppHandle, step: u8, label: &str, percent: u8) {
    let _ = app.emit(
        "init-progress",
        InitProgress {
            step,
            label: label.to_string(),
            percent,
        },
    );
}

/// 分步初始化工作区，推送 init-progress 事件。
pub fn init_with_progress(app: &AppHandle) -> DstResult<String> {
    if workspace::is_initialized() {
        return Err(DstError::WorkspaceExists(
            workspace::workspace_root().display().to_string(),
        ));
    }

    emit_progress(app, 1, "写入模板文件…", 20);
    workspace::init_from_template()?;

    emit_progress(app, 2, "创建 Studio 元数据…", 40);

    emit_progress(app, 3, "初始化 Git 仓库…", 70);
    let r = workspace::workspace_root();
    git::init_repo(&r)?;
    workspace::touch_last_sync()?;

    emit_progress(app, 4, "完成", 100);
    git::head_oid(&r)
}
