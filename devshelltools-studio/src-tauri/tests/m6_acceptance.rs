mod common;

#[cfg(test)]
mod tests {
    use crate::common::IsolatedProfile;
    use devshelltools_studio_lib::function_edit::{self, FunctionDraft};
    use devshelltools_studio_lib::install_mgr;
    use devshelltools_studio_lib::workspace;

    fn setup_workspace() {
        if !workspace::is_initialized() {
            workspace::init_from_template().expect("init");
        }
    }

    #[test]
    fn m6_upsert_and_delete_function() {
        let _g = IsolatedProfile::new("m6-fn");
        setup_workspace();

        let draft = FunctionDraft {
            name: "m6test".into(),
            synopsis: "M6 测试命令".into(),
            example: "m6test".into(),
            body: None,
        };
        function_edit::upsert_function("Files.ps1", draft).expect("upsert");

        let content = workspace::read_file("Public/Files.ps1").expect("read");
        assert!(content.contains("function m6test"));

        function_edit::delete_function("Files.ps1", "m6test").expect("delete");
        let content2 = workspace::read_file("Public/Files.ps1").expect("read2");
        assert!(!content2.contains("function m6test"));
    }

    /// 工作区 == PS5.1 模块目录时，安装不得清空自身，且应能 Import-Module。
    #[test]
    fn m6_install_does_not_destroy_workspace_when_source_equals_ps51() {
        let _g = IsolatedProfile::new("m6-install-preserve");
        setup_workspace();

        let root = workspace::workspace_root();
        let psd1 = root.join("DevShellTools.psd1");
        assert!(psd1.exists(), "安装前工作区应有清单");
        assert_eq!(
            root,
            install_mgr::ps51_module_dir(),
            "Studio 工作区必须等于 PS5.1 模块目录（本 bug 的前提）"
        );

        let result = install_mgr::install_module().expect("install_module 应成功");
        assert!(
            psd1.exists(),
            "安装后工作区 DevShellTools.psd1 必须仍存在（禁止 source==target 自毁）"
        );
        assert!(
            workspace::is_initialized(),
            "安装后工作区仍应处于已初始化状态"
        );
        assert!(result.status.profile_configured);
        assert!(result.status.ps7_module_present);
        assert!(result.status.installed);
        assert!(result.verified, "安装后 status.installed 应被验证为 true");
    }

    #[test]
    fn m6_install_status_and_soft_uninstall() {
        let _g = IsolatedProfile::new("m6-install");
        setup_workspace();

        let before = install_mgr::install_status();
        assert!(before.workspace_ready);
        assert!(before.ps51_module_present);

        let after_install = install_mgr::install_module().expect("install");
        assert!(after_install.status.profile_configured);
        assert!(after_install.status.ps7_module_present);
        assert!(
            workspace::is_initialized(),
            "安装后工作区应保留"
        );

        let after_uninstall = install_mgr::uninstall_module().expect("uninstall");
        assert!(!after_uninstall.status.installed);
        assert!(
            workspace::is_initialized(),
            "软卸载后工作区应保留（不得删除含 .studio 的 PS5.1 目录）"
        );
        assert!(
            workspace::workspace_root()
                .join("DevShellTools.psd1")
                .exists(),
            "软卸载后清单文件应保留"
        );
    }

    #[test]
    fn m6_isolation_uses_temp_documents_not_real() {
        let g = IsolatedProfile::new("m6-iso");
        let root = workspace::workspace_root();
        let root_s = root.to_string_lossy().to_lowercase();
        let docs = g.documents_dir().to_string_lossy().to_lowercase();
        assert!(
            root_s.starts_with(&docs),
            "隔离后工作区应落在临时 Documents 下：root={root_s} docs={docs}"
        );
        assert!(
            root_s.contains("dst-m6-iso-profile"),
            "工作区路径应包含隔离前缀：{root_s}"
        );
    }
}
