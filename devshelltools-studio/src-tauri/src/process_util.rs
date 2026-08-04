use std::process::{Command, Output, Stdio};

/// Windows CREATE_NO_WINDOW
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 配置 Command 为无窗口静默执行（Windows）。
pub fn configure_hidden(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.stdin(Stdio::null());
}

/// 静默执行并捕获输出。
pub fn output_hidden(mut cmd: Command) -> std::io::Result<Output> {
    configure_hidden(&mut cmd);
    cmd.output()
}

/// 静默执行（接受 &mut Command，用于链式调用场景）。
pub fn output_hidden_ref(cmd: &mut Command) -> std::io::Result<Output> {
    configure_hidden(cmd);
    cmd.output()
}

/// PowerShell 通用参数前缀（隐藏窗口 + 非交互）。
pub fn ps_base_args(exe: &str) -> Vec<String> {
    let mut args = vec![
        "-NoProfile".to_string(),
        "-NonInteractive".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
    ];
    if exe.eq_ignore_ascii_case("powershell") || exe.eq_ignore_ascii_case("powershell.exe") {
        args.push("-WindowStyle".to_string());
        args.push("Hidden".to_string());
    }
    args
}
