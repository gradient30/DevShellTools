pub mod commands;
pub mod consistency;
pub mod error;
pub mod git;
pub mod ps_parser;
pub mod safety;
pub mod sync;
pub mod template;
pub mod workspace;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            // 工作区
            commands::workspace_status,
            commands::init_workspace,
            // 读取
            commands::list_public_files,
            commands::read_workspace_file,
            commands::list_categories,
            commands::read_category_file,
            // 写入 / CRUD
            commands::write_workspace_file,
            commands::delete_workspace_file,
            commands::create_category,
            commands::delete_category,
            commands::update_category_file,
            commands::sync_public,
            // 校验
            commands::consistency_check,
            commands::safety_check,
            commands::validate_ps_syntax,
            // Git
            commands::git_log,
            commands::git_reset_hard,
            commands::git_snapshot,
        ])
        .run(tauri::generate_context!())
        .expect("启动 DevShellTools Studio 失败");
}