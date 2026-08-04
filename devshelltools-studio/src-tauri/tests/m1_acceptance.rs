mod common;

/// M1 端到端验收：模拟首次启动初始化工作区 + git 首次提交。
/// 用临时 USERPROFILE 隔离，不影响真实工作区。
#[test]
fn m1_acceptance_init_workspace_and_git_commit() {
    let _profile = common::IsolatedProfile::new("m1");

    // 1. 初始状态：未初始化
    assert!(
        !devshelltools_studio_lib::workspace::is_initialized(),
        "临时目录下不应已有工作区"
    );

    // 2. 从模板初始化
    devshelltools_studio_lib::workspace::init_from_template()
        .expect("init_from_template 失败");

    // 3. 校验核心文件存在
    assert!(devshelltools_studio_lib::workspace::is_initialized());
    let root = devshelltools_studio_lib::workspace::workspace_root();
    assert!(root.join("DevShellTools.psd1").exists());
    assert!(root.join("DevShellTools.psm1").exists());
    assert!(root.join("Private").join("Common.ps1").exists());
    assert!(root.join("Public").join("Files.ps1").exists());
    assert!(root.join("Public").join("Help.ps1").exists());
    assert!(root.join("install.ps1").exists());

    // 4. 校验元数据文件
    assert!(devshelltools_studio_lib::workspace::meta_file().exists());

    // 5. git init + 首次提交
    devshelltools_studio_lib::git::init_repo(&root).expect("git init 失败");
    assert!(devshelltools_studio_lib::git::is_repo(&root));

    // 6. 校验首次提交存在
    let oid = devshelltools_studio_lib::git::head_oid(&root).expect("head_oid 失败");
    assert!(!oid.is_empty());

    // 7. 校验提交记录
    let log = devshelltools_studio_lib::git::log(&root, 5).expect("log 失败");
    assert!(!log.is_empty());
    assert!(log[0].message.contains("init") || log[0].message.contains("template"));

    // 8. 模拟变更 + 快照
    std::fs::write(root.join("Public").join("test-new.ps1"), "# test\n").unwrap();
    let oid2 = devshelltools_studio_lib::git::snapshot(&root, "add test file")
        .expect("snapshot 失败");
    assert_ne!(oid, oid2);

    let log2 = devshelltools_studio_lib::git::log(&root, 5).expect("log2 失败");
    assert!(log2.len() >= 2);
    assert_eq!(log2[0].message, "add test file");

    // 9. 校验 status 接口
    let status = devshelltools_studio_lib::workspace::status().expect("status 失败");
    assert!(status.initialized);
    assert_eq!(status.version, "1.0.5");
    assert!(status.public_files.contains(&"Files.ps1".to_string()));
    assert!(status.public_files.contains(&"test-new.ps1".to_string()));
}
