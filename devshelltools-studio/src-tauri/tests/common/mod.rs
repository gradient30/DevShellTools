//! 验收测试共享环境：串行化 USERPROFILE 隔离，避免并行测试互相污染。

use std::env;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static USERPROFILE_LOCK: Mutex<()> = Mutex::new(());

pub struct IsolatedProfile {
    _lock: std::sync::MutexGuard<'static, ()>,
    tmp: PathBuf,
    original: String,
}

impl IsolatedProfile {
    pub fn new(prefix: &str) -> Self {
        let _lock = USERPROFILE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let original = env::var("USERPROFILE").unwrap_or_default();
        let tmp = make_tmp_user_profile(prefix);
        env::set_var("USERPROFILE", tmp.to_str().unwrap());
        Self {
            _lock,
            tmp,
            original,
        }
    }
}

impl Drop for IsolatedProfile {
    fn drop(&mut self) {
        env::set_var("USERPROFILE", &self.original);
        let _ = std::fs::remove_dir_all(&self.tmp);
    }
}

fn make_tmp_user_profile(prefix: &str) -> PathBuf {
    let mut p = env::temp_dir();
    p.push(format!(
        "dst-{prefix}-profile-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}
