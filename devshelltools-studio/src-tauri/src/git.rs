use crate::error::{DstError, DstResult};
use std::path::Path;
use std::process::Command;

/// 工作区 git 是否已 init。
pub fn is_repo(root: &Path) -> bool {
    run_git(root, &["rev-parse", "--is-inside-work-tree"]).is_ok()
}

/// 初始化工作区为 git 仓库。已存在则跳过。
pub fn init_repo(root: &Path) -> DstResult<()> {
    if is_repo(root) {
        return Ok(());
    }
    // .git 可能残留且非有效仓库（如 hook 文件冲突），清理后重新 init
    let dot_git = root.join(".git");
    if dot_git.exists() {
        std::fs::remove_dir_all(&dot_git).ok();
    }
    run_git(root, &["init"])?;
    run_git(root, &["config", "user.name", "DevShellTools Studio"])?;
    run_git(root, &["config", "user.email", "studio@devshelltools.local"])?;
    add_all(root)?;
    run_git(
        root,
        &["commit", "-m", "init workspace from template v1.0.5"],
    )?;
    Ok(())
}

/// add -A
pub fn add_all(root: &Path) -> DstResult<()> {
    run_git(root, &["add", "-A"])?;
    Ok(())
}

/// 把工作区所有变更作为一次 commit。返回新提交 oid（hex）。
pub fn snapshot(root: &Path, message: &str) -> DstResult<String> {
    if !is_repo(root) {
        init_repo(root)?;
    }
    add_all(root)?;
    let has_changes = run_git(root, &["status", "--porcelain"])
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let head_exists = run_git(root, &["rev-parse", "--verify", "HEAD"]).is_ok();

    if !has_changes && head_exists {
        return head_oid(root);
    }
    if !head_exists {
        run_git(root, &["commit", "--allow-empty", "-m", message])?;
    } else {
        run_git(root, &["commit", "-m", message])?;
    }
    head_oid(root)
}

/// HEAD 提交 oid（hex 字符串）。
pub fn head_oid(root: &Path) -> DstResult<String> {
    let out = run_git(root, &["rev-parse", "HEAD"])?;
    Ok(out.trim().to_string())
}

/// 最近 N 条提交记录。
pub fn log(root: &Path, n: usize) -> DstResult<Vec<CommitInfo>> {
    let n_str = n.to_string();
    let out = run_git(
        root,
        &["log", "-n", &n_str, "--pretty=format:%H%x1f%s%x1f%ct"],
    )?;
    let mut result = vec![];
    for line in out.lines() {
        let parts: Vec<&str> = line.split('\x1f').collect();
        if parts.len() == 3 {
            result.push(CommitInfo {
                oid: parts[0].to_string(),
                message: parts[1].to_string(),
                time: parts[2].parse().unwrap_or(0),
            });
        }
    }
    Ok(result)
}

/// 把整个工作区回到某个提交。
pub fn reset_hard(root: &Path, oid_hex: &str) -> DstResult<()> {
    run_git(root, &["reset", "--hard", oid_hex])?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CommitInfo {
    pub oid: String,
    pub message: String,
    pub time: i64,
}

fn run_git(root: &Path, args: &[&str]) -> DstResult<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| DstError::Other(format!("启动 git 失败：{e}")))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(DstError::Other(format!(
            "git {} 失败：{}",
            args.join(" "),
            err.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_and_snapshot_roundtrip() {
        let root = test_tmp().join("ws");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("a.txt"), "init").unwrap();

        init_repo(&root).expect("init");
        assert!(is_repo(&root));
        let oid1 = snapshot(&root, "first").expect("snap1");
        std::fs::write(root.join("b.txt"), "second").unwrap();
        let oid2 = snapshot(&root, "second").expect("snap2");
        assert_ne!(oid1, oid2);

        let log = log(&root, 10).expect("log");
        assert!(log.len() >= 2);

        reset_hard(&root, &oid1).expect("reset");
        assert!(root.join("a.txt").exists());
        assert!(!root.join("b.txt").exists());

        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }

    fn test_tmp() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "dst-git-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}