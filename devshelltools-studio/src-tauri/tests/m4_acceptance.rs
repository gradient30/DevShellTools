#![allow(dead_code)]

#[cfg(test)]
mod tests {
    use devshelltools_studio_lib::export;
    use devshelltools_studio_lib::logging;
    use devshelltools_studio_lib::migrate;
    use devshelltools_studio_lib::webview2;
    use std::env;
    use std::path::PathBuf;

    fn isolated_profile() -> PathBuf {
        let original = env::var("USERPROFILE").unwrap_or_default();
        let mut p = env::temp_dir();
        p.push(format!(
            "dst-m4-profile-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        env::set_var("USERPROFILE", p.to_str().unwrap());
        p
    }

    /// 验收1：迁移检测返回结构正确
    #[test]
    fn m4_migration_check_struct() {
        let _g = isolated_profile();
        let check = migrate::check_migration();
        assert!(check.legacy_dirs.len() <= 2, "最多 2 个旧版目录");
    }

    /// 验收2：导出/导入 round-trip
    #[test]
    fn m4_export_import_roundtrip() {
        let _g = isolated_profile();
        // 初始化工作区
        devshelltools_studio_lib::workspace::init_from_template().expect("init");
        let root = devshelltools_studio_lib::workspace::workspace_root();
        devshelltools_studio_lib::git::init_repo(&root).expect("git init");

        // 创建测试文件
        devshelltools_studio_lib::workspace::write_file("Public/TestExport.ps1", "# test export\n").unwrap();
        devshelltools_studio_lib::git::snapshot(&root, "test: add export file").unwrap();

        // 导出
        let export_dir = root.parent().unwrap().join("dst-export-target");
        let path = export::export_to(export_dir.to_str().unwrap()).expect("export");
        assert!(export_dir.join("DevShellTools.psd1").exists(), "导出应含 psd1");
        assert!(export_dir.join("Public").join("TestExport.ps1").exists(), "导出应含测试文件");
        assert!(!export_dir.join(".git").exists(), "导出不含 .git");

        // 导入到新位置（模拟多机）
        drop(_g);
        let _g2 = isolated_profile();
        let root2 = devshelltools_studio_lib::workspace::workspace_root();
        // 初始化新工作区
        devshelltools_studio_lib::workspace::init_from_template().expect("init2");
        devshelltools_studio_lib::git::init_repo(&root2).expect("git init2");

        let files = export::import_from(export_dir.to_str().unwrap()).expect("import");
        assert!(files.len() > 0, "应导入文件");
        assert!(root2.join("Public").join("TestExport.ps1").exists(), "导入后应含测试文件");
    }

    /// 验收3：日志脱敏
    #[test]
    fn m4_log_sanitization() {
        let _g = isolated_profile();
        // sk- key 脱敏
        let s1 = logging::sanitize("api_key=sk-abc1234567890xyz");
        assert!(s1.contains("****"));
        assert!(!s1.contains("7890xyz"));
        // Bearer 脱敏
        let s2 = logging::sanitize("Authorization: Bearer abcdef12345678");
        assert!(s2.contains("****"));
        assert!(!s2.contains("abcdef12345678"));
        // 普通文本不变
        let s3 = logging::sanitize("普通日志消息");
        assert_eq!(s3, "普通日志消息");
    }

    /// 验收4：WebView2 状态检测不 panic
    #[test]
    fn m4_webview2_check_no_panic() {
        let _g = isolated_profile();
        let status = webview2::check_webview2().expect("webview2 check");
        // 测试环境可能装了也可能没装，只要不 panic 即可
        assert!(status.installed || !status.installed); // 总是 true，验证不 panic
    }

    /// 验收5：导出排除 .studio 目录
    #[test]
    fn m4_export_excludes_studio() {
        let _g = isolated_profile();
        devshelltools_studio_lib::workspace::init_from_template().expect("init");
        let root = devshelltools_studio_lib::workspace::workspace_root();
        devshelltools_studio_lib::git::init_repo(&root).expect("git init");

        let export_dir = root.parent().unwrap().join("dst-export-exclude-test");
        export::export_to(export_dir.to_str().unwrap()).expect("export");
        assert!(!export_dir.join(".studio").exists(), "导出应排除 .studio");
        assert!(!export_dir.join(".git").exists(), "导出应排除 .git");
    }
}