pub mod commands;
pub mod error;
pub mod git;
pub mod template;
pub mod workspace;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            commands::workspace_status,
            commands::init_workspace,
            commands::list_public_files,
            commands::read_workspace_file,
            commands::write_workspace_file,
            commands::delete_workspace_file,
            commands::git_log,
            commands::git_reset_hard,
            commands::git_snapshot,
        ])
        .run(tauri::generate_context!())
        .expect("启动 DevShellTools Studio 失败");
}