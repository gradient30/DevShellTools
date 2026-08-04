use crate::ai_client;
use crate::ai_config::{self, AiConfig, AiProfile, ChatMessage};
use crate::consistency;
use crate::error::{DstError, DstResult};
use crate::export;
use crate::function_edit::{self, FunctionDraft, FunctionTestResult};
use crate::git;
use crate::init_progress;
use crate::install_mgr;
use crate::logging;
use crate::migrate;
use crate::ps_parser;
use crate::safety;
use crate::sync::{self, CategoryInfo};
use crate::webview2;
use crate::workspace;
use tauri::AppHandle;

fn root() -> std::path::PathBuf {
    workspace::workspace_root()
}

// ============ 工作区管理 ============

#[tauri::command]
pub fn workspace_status() -> DstResult<workspace::WorkspaceStatus> {
    workspace::status()
}

#[tauri::command]
pub fn init_workspace(app: AppHandle) -> DstResult<String> {
    init_progress::init_with_progress(&app)
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
    sync::scan_categories_cached()
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
    sync::invalidate_category_cache();
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
    sync::invalidate_category_cache();
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
    sync::invalidate_category_cache();
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
    sync::invalidate_category_cache();
    let r = root();
    let oid = git::snapshot(&r, &message)?;
    workspace::touch_last_sync()?;
    Ok(oid)
}

// ============ 函数级 CRUD ============

#[tauri::command]
pub fn upsert_function(
    file_name: String,
    name: String,
    synopsis: String,
    example: String,
    body: Option<String>,
    message: String,
) -> DstResult<String> {
    function_edit::upsert_function(
        &file_name,
        FunctionDraft {
            name,
            synopsis,
            example,
            body,
        },
        &message,
    )
}

#[tauri::command]
pub fn delete_function(
    file_name: String,
    func_name: String,
    message: String,
) -> DstResult<String> {
    function_edit::delete_function(&file_name, &func_name, &message)
}

#[tauri::command]
pub fn test_function(file_name: String, func_name: String) -> DstResult<FunctionTestResult> {
    function_edit::test_function(&file_name, &func_name)
}

#[tauri::command]
pub fn apply_ai_code(
    file_name: String,
    code: String,
    message: String,
) -> DstResult<Vec<String>> {
    function_edit::apply_code_to_category(&file_name, &code, &message)
}

// ============ 安装 / 卸载 ============

#[tauri::command]
pub fn install_status() -> install_mgr::InstallStatus {
    install_mgr::install_status()
}

#[tauri::command]
pub fn install_module() -> DstResult<install_mgr::InstallResult> {
    install_mgr::install_module()
}

#[tauri::command]
pub fn uninstall_module() -> DstResult<install_mgr::InstallResult> {
    install_mgr::uninstall_module()
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

/// 检测 AI 是否就绪（任一 Profile 有 key）。
#[tauri::command]
pub fn ai_ready() -> bool {
    ai_config::is_configured()
}

#[tauri::command]
pub fn list_ai_profiles() -> DstResult<Vec<AiProfile>> {
    ai_config::list_profiles()
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AiProfilesMeta {
    pub profiles: Vec<AiProfile>,
    pub default_profile_id: Option<String>,
}

#[tauri::command]
pub fn get_ai_profiles_meta() -> DstResult<AiProfilesMeta> {
    let store = ai_config::load_profiles_store()?;
    Ok(AiProfilesMeta {
        profiles: store.profiles,
        default_profile_id: store.default_profile_id,
    })
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SaveAiProfileInput {
    pub profile: AiProfile,
    pub key: Option<String>,
}

#[tauri::command]
pub fn save_ai_profile(input: SaveAiProfileInput) -> DstResult<AiProfile> {
    ai_config::save_profile(input.profile, input.key.as_deref())
}

#[tauri::command]
pub fn delete_ai_profile(id: String) -> DstResult<()> {
    ai_config::delete_profile(&id)
}

#[tauri::command]
pub fn set_default_ai_profile(id: String) -> DstResult<()> {
    let mut store = ai_config::load_profiles_store()?;
    if !store.profiles.iter().any(|p| p.id == id) {
        return Err(DstError::Other(format!("Profile 不存在：{id}")));
    }
    store.default_profile_id = Some(id);
    ai_config::save_profiles_store(&store)
}

#[tauri::command]
pub async fn test_ai_profile(id: String) -> DstResult<String> {
    let config = ai_config::load_config_for_profile(Some(&id))?;
    let api_key = ai_config::load_key_for_profile(&id)?;
    let messages = vec![ChatMessage {
        role: "user".into(),
        content: "回复 OK".into(),
    }];
    let events = ai_client::chat_stream(&config, &api_key, messages).await?;
    let full: String = events
        .iter()
        .filter(|e| e.kind == "delta")
        .map(|e| e.content.as_str())
        .collect();
    if full.trim().is_empty() {
        return Err(DstError::Other("模型返回为空".into()));
    }
    Ok(full.chars().take(120).collect())
}

#[tauri::command]
pub fn list_ai_presets() -> Vec<crate::ai_presets::AiPreset> {
    crate::ai_presets::list_presets()
}

#[tauri::command]
pub fn suggest_ai_endpoint(
    protocol: ai_config::AiProtocol,
    current_base_url: Option<String>,
) -> crate::ai_presets::AiEndpointSuggestion {
    crate::ai_presets::suggest_endpoint(protocol, current_base_url.as_deref())
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FetchModelsInput {
    pub protocol: ai_config::AiProtocol,
    pub base_url: String,
    pub key: String,
}

#[tauri::command]
pub async fn fetch_ai_models(id: String) -> DstResult<Vec<String>> {
    let config = ai_config::load_config_for_profile(Some(&id))?;
    let api_key = ai_config::load_key_for_profile(&id)?;
    ai_client::list_models(&config, &api_key).await
}

/// 拉取模型预览（不持久化 Profile，供编辑对话框使用）。
#[tauri::command]
pub async fn fetch_ai_models_preview(input: FetchModelsInput) -> DstResult<Vec<String>> {
    if input.key.trim().is_empty() {
        return Err(DstError::Other("请先填写 API Key".into()));
    }
    let config = AiConfig {
        protocol: input.protocol,
        base_url: input.base_url,
        model: String::new(),
        temperature: 0.7,
        max_tokens: 2048,
    };
    ai_client::list_models(&config, input.key.trim()).await
}

// ============ AI 对话 ============

async fn ai_chat_inner(messages: Vec<ChatMessage>, profile_id: Option<String>) -> DstResult<String> {
    let config = ai_config::load_config_for_profile(profile_id.as_deref())?;
    let id = profile_id
        .or_else(|| ai_config::load_profiles_store().ok()?.default_profile_id)
        .ok_or_else(|| DstError::Other("未选择 AI Profile".into()))?;
    let api_key = ai_config::load_key_for_profile(&id)?;
    let events = ai_client::chat_stream(&config, &api_key, messages).await?;
    let full: String = events
        .iter()
        .filter(|e| e.kind == "delta")
        .map(|e| e.content.as_str())
        .collect();
    Ok(full)
}

/// 发起一次 AI 对话（非流式，一次返回全部 delta）。
#[tauri::command]
pub async fn ai_chat(messages: Vec<ChatMessage>, profile_id: Option<String>) -> DstResult<String> {
    ai_chat_inner(messages, profile_id).await
}

/// AI 对话并自动校验生成的代码。
#[tauri::command]
pub async fn ai_chat_with_validation(
    messages: Vec<ChatMessage>,
    profile_id: Option<String>,
) -> DstResult<AiChatResult> {
    let reply = ai_chat_inner(messages, profile_id).await?;
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

// ============ M4：迁移 / 导出导入 / 日志 / WebView2 ============

/// 检测旧版 install.ps1 安装。
#[tauri::command]
pub fn check_migration() -> migrate::MigrationCheck {
    migrate::check_migration()
}

/// 执行迁移：从旧版安装合并命令到便携工作区。
#[tauri::command]
pub fn migrate_legacy() -> DstResult<Vec<String>> {
    let files = migrate::migrate_from_legacy()?;
    logging::log(logging::LogLevel::Info, "migrate", &format!("迁移 {} 个文件", files.len()));
    Ok(files)
}

/// 导出工作区到目录。
#[tauri::command]
pub fn export_workspace(target_dir: String) -> DstResult<String> {
    let path = export::export_to(&target_dir)?;
    logging::log(logging::LogLevel::Info, "export", &format!("导出到 {path}"));
    Ok(path)
}

/// 从目录导入工作区。
#[tauri::command]
pub fn import_workspace(source_dir: String) -> DstResult<Vec<String>> {
    let files = export::import_from(&source_dir)?;
    sync::invalidate_category_cache();
    logging::log(logging::LogLevel::Info, "import", &format!("导入 {} 项", files.len()));
    Ok(files)
}

/// 列出日志文件。
#[tauri::command]
pub fn list_logs() -> Vec<String> {
    logging::list_log_files()
}

/// 读取指定日志文件内容。
#[tauri::command]
pub fn read_log(name: String) -> DstResult<String> {
    Ok(logging::read_log_file(&name))
}

/// WebView2 Runtime 状态。
#[tauri::command]
pub fn webview2_status() -> DstResult<webview2::Webview2Status> {
    webview2::check_webview2()
}

/// WebView2 下载 URL。
#[tauri::command]
pub fn webview2_download_url() -> String {
    webview2::WEBVIEW2_DOWNLOAD_URL.to_string()
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