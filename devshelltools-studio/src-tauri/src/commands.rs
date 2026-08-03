use crate::ai_client;
use crate::ai_config::{self, AiConfig, ChatMessage};
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

// ============ AI 配置 ============

/// 读取 AI 配置。
#[tauri::command]
pub fn get_ai_config() -> DstResult<AiConfig> {
    ai_config::load_config()
}

/// 保存 AI 配置。
#[tauri::command]
pub fn save_ai_config(config: AiConfig) -> DstResult<()> {
    ai_config::save_config(&config)
}

/// 保存 API Key（前端传明文，后端写文件）。
#[tauri::command]
pub fn save_ai_key(key: String) -> DstResult<()> {
    ai_config::save_key(&key)
}

/// 读取 API Key（前端展示用，返回是否已配置 + 掩码）。
#[tauri::command]
pub fn get_ai_key_status() -> DstResult<AiKeyStatus> {
    match ai_config::load_key() {
        Ok(k) => {
            let masked = if k.len() > 8 {
                format!("{}...{}", &k[..4], &k[k.len() - 4..])
            } else {
                "****".into()
            };
            Ok(AiKeyStatus { configured: true, masked })
        }
        Err(_) => Ok(AiKeyStatus {
            configured: false,
            masked: String::new(),
        }),
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiKeyStatus {
    pub configured: bool,
    pub masked: String,
}

/// 检测 AI 是否就绪（配置 + key 都存在）。
#[tauri::command]
pub fn ai_ready() -> bool {
    ai_config::is_configured()
}

// ============ AI 对话 ============

/// 发起一次 AI 对话（非流式，一次返回全部 delta）。
/// 前端传历史消息，后端注入 system prompt 后请求 AI。
#[tauri::command]
pub async fn ai_chat(messages: Vec<ChatMessage>) -> DstResult<String> {
    let config = ai_config::load_config()?;
    let api_key = ai_config::load_key()?;
    let events = ai_client::chat_stream(&config, &api_key, messages).await?;
    // 拼接所有 delta
    let full: String = events
        .iter()
        .filter(|e| e.kind == "delta")
        .map(|e| e.content.as_str())
        .collect();
    Ok(full)
}

/// AI 对话并自动校验生成的代码。
/// 返回 AI 回复 + 提取的代码块 + 每个代码块的安全/语法校验结果。
#[tauri::command]
pub async fn ai_chat_with_validation(
    messages: Vec<ChatMessage>,
) -> DstResult<AiChatResult> {
    let reply = ai_chat(messages).await?;
    let code_blocks = ai_config::extract_code_blocks(&reply);

    let mut validated = vec![];
    for code in &code_blocks {
        let syntax_ok = ps_parser::validate_syntax(code).is_ok();
        let syntax_err = match ps_parser::validate_syntax(code) {
            Ok(_) => String::new(),
            Err(e) => e.to_string(),
        };
        let safety_report = safety::check(code).unwrap_or(safety::SafetyReport {
            ok: false,
            violations: vec!["安全检查内部错误".into()],
        });
        let parsed = ps_parser::parse_ps1(code).ok();
        let (functions, category) = parsed
            .as_ref()
            .map(|p| {
                let fns: Vec<String> = p.functions.iter().map(|f| f.name.clone()).collect();
                let cat = p.category.as_ref().map(|c| c.name.clone());
                (fns, cat)
            })
            .unwrap_or_default();
        validated.push(ValidatedCodeBlock {
            code: code.clone(),
            syntax_ok,
            syntax_err,
            safety_ok: safety_report.ok,
            safety_violations: safety_report.violations,
            functions,
            category,
        });
    }

    Ok(AiChatResult { reply, code_blocks: validated })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidatedCodeBlock {
    pub code: String,
    pub syntax_ok: bool,
    pub syntax_err: String,
    pub safety_ok: bool,
    pub safety_violations: Vec<String>,
    pub functions: Vec<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiChatResult {
    pub reply: String,
    pub code_blocks: Vec<ValidatedCodeBlock>,
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