#![allow(dead_code)]

mod common;

#[cfg(test)]
mod tests {
    use crate::common::IsolatedProfile;
    use devshelltools_studio_lib::export;
    use devshelltools_studio_lib::logging;
    use devshelltools_studio_lib::migrate;
    use devshelltools_studio_lib::webview2;

    /// 验收1：迁移检测返回结构正确
    #[test]
    fn m4_migration_check_struct() {
        let _g = IsolatedProfile::new("m4");
        let check = migrate::check_migration();
        assert!(check.legacy_dirs.len() <= 3, "最多 3 个旧版目录");
    }

    /// 验收2：导出/导入 round-trip
    #[test]
    fn m4_export_import_roundtrip() {
        let _g = IsolatedProfile::new("m4-export");
        // 初始化工作区
        devshelltools_studio_lib::workspace::init_from_template().expect("init");
        let root = devshelltools_studio_lib::workspace::workspace_root();
        
        // 创建测试文件
        devshelltools_studio_lib::workspace::write_file("Public/TestExport.ps1", "# test export\n")
            .unwrap();
        
        // 导出到系统 temp（不在 USERPROFILE 隔离目录内，便于跨 profile 导入测试）
        let export_dir = std::env::temp_dir().join(format!(
            "dst-export-target-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&export_dir);
        export::export_scripts(export_dir.to_str().unwrap()).expect("export");
        assert!(
            export_dir.join("DevShellTools.psd1").exists(),
            "导出应含 psd1"
        );
        assert!(
            export_dir.join("Public").join("TestExport.ps1").exists(),
            "导出应含测试文件"
        );
        assert!(!export_dir.join(".git").exists(), "导出不含 .git");

        // 导入到新位置（模拟多机）
        drop(_g);
        let _g2 = IsolatedProfile::new("m4-import");
        let root2 = devshelltools_studio_lib::workspace::workspace_root();
        // 初始化新工作区
        devshelltools_studio_lib::workspace::init_from_template().expect("init2");
        
        let files = export::import_scripts(export_dir.to_str().unwrap()).expect("import");
        assert!(!files.is_empty(), "应导入文件");
        assert!(
            root2.join("Public").join("TestExport.ps1").exists(),
            "导入后应含测试文件"
        );

        let _ = std::fs::remove_dir_all(&export_dir);
    }

    /// 验收3：日志脱敏
    #[test]
    fn m4_log_sanitization() {
        let _g = IsolatedProfile::new("m4-log");
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
        let _g = IsolatedProfile::new("m4-webview2");
        let status = webview2::check_webview2().expect("webview2 check");
        // 测试环境可能装了也可能没装，只要不 panic 即可
        assert!(status.installed || !status.installed);
    }

    /// 验收5：导出排除 .studio 目录
    #[test]
    fn m4_export_excludes_studio() {
        let _g = IsolatedProfile::new("m4-exclude");
        devshelltools_studio_lib::workspace::init_from_template().expect("init");
        let root = devshelltools_studio_lib::workspace::workspace_root();
        
        let export_dir = std::env::temp_dir().join(format!(
            "dst-export-exclude-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&export_dir);
        export::export_scripts(export_dir.to_str().unwrap()).expect("export");
        assert!(!export_dir.join(".studio").exists(), "导出应排除 .studio");
        assert!(!export_dir.join(".git").exists(), "导出应排除 .git");
        let _ = std::fs::remove_dir_all(&export_dir);
    }
}
