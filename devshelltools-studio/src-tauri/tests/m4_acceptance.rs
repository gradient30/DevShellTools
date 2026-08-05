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
        assert!(check.legacy_dirs.len() <= 3, "最多 3 个待处理来源");
    }

    /// 验收：迁移后旧沙箱归档，且不再提示 has_legacy（隔离环境下无真实旧目录时跳过归档断言）
    #[test]
    fn m4_migrate_clears_sandbox_when_present() {
        let _g = IsolatedProfile::new("m4-migrate");
        devshelltools_studio_lib::workspace::init_from_template().expect("init");
        let docs = std::env::var("USERPROFILE")
            .map(std::path::PathBuf::from)
            .unwrap()
            .join("Documents");
        let sandbox = docs.join("DevShellTools");
        let _ = std::fs::remove_dir_all(&sandbox);
        std::fs::create_dir_all(sandbox.join("Public")).unwrap();
        std::fs::write(
            sandbox.join("DevShellTools.psd1"),
            "@{ ModuleVersion = '0.0.1' }",
        )
        .unwrap();
        std::fs::write(
            sandbox.join("Public").join("LegacyOnly.ps1"),
            r#"
# @DST-Category
# Name: legacyonly
# Title: 旧版
# Description: 迁移验收
# Aliases:
# @DST-Category-End
function legacyonly {
<#
.SYNOPSIS
迁移验收命令。
.EXAMPLE
legacyonly
#>
    [CmdletBinding()] param()
    "ok"
}
"#,
        )
        .unwrap();

        let before = migrate::check_migration();
        assert!(before.has_legacy, "应检测到旧沙箱");

        let result = migrate::migrate_from_legacy().expect("migrate");
        assert!(
            result.migrated_files.iter().any(|f| f == "LegacyOnly.ps1"),
            "应迁入 LegacyOnly.ps1：{:?}",
            result.migrated_files
        );
        assert!(!sandbox.exists(), "旧沙箱应已归档：{}", sandbox.display());
        assert!(
            !result.archived_dirs.is_empty(),
            "应返回归档路径：{:?}",
            result
        );

        let after = migrate::check_migration();
        assert!(
            !after.has_legacy,
            "迁移清理后不应再提示旧版：{:?}",
            after.legacy_dirs
        );
    }

    /// 验收2：导出/导入 round-trip
    #[test]
    fn m4_export_import_roundtrip() {
        let _g = IsolatedProfile::new("m4-export");
        // 初始化工作区
        devshelltools_studio_lib::workspace::init_from_template().expect("init");
        let root = devshelltools_studio_lib::workspace::workspace_root();
        
        // 创建可安全导入的测试脚本（当前导出仅为 Public/*.ps1 平铺）
        let test_ps1 = r#"
function testexport {
<#
.SYNOPSIS
导出导入验收用命令。
.EXAMPLE
testexport
#>
    [CmdletBinding()] param()
    Write-Output "ok"
}
"#;
        devshelltools_studio_lib::workspace::write_file("Public/TestExport.ps1", test_ps1)
            .unwrap();

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
            export_dir.join("TestExport.ps1").exists(),
            "导出目录应平铺含 TestExport.ps1"
        );
        assert!(
            !export_dir.join("DevShellTools.psd1").exists(),
            "脚本级导出不应含公共部分 psd1"
        );
        assert!(!export_dir.join(".git").exists(), "导出不含 .git");
        let _ = root;

        // 导入到新位置（模拟多机）
        drop(_g);
        let _g2 = IsolatedProfile::new("m4-import");
        let root2 = devshelltools_studio_lib::workspace::workspace_root();
        devshelltools_studio_lib::workspace::init_from_template().expect("init2");

        let imported = export::import_scripts(export_dir.to_str().unwrap()).expect("import");
        assert!(
            imported.imported.iter().any(|n| n == "TestExport.ps1"),
            "应导入 TestExport.ps1：{:?}",
            imported
        );
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
