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
            let root = workspace::workspace_root();
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
        function_edit::upsert_function("Files.ps1", draft, "test: add m6test").expect("upsert");

        let content = workspace::read_file("Public/Files.ps1").expect("read");
        assert!(content.contains("function m6test"));

        function_edit::delete_function("Files.ps1", "m6test", "test: del m6test").expect("delete");
        let content2 = workspace::read_file("Public/Files.ps1").expect("read2");
        assert!(!content2.contains("function m6test"));
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

        let after_uninstall = install_mgr::uninstall_module().expect("uninstall");
        assert!(!after_uninstall.status.installed);
        assert!(workspace::is_initialized(), "软卸载后工作区应保留");
    }
}
