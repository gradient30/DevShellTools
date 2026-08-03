use crate::error::{DstError, DstResult};
use crate::workspace;
use serde::{Deserialize, Serialize};

/// AI 提供商协议
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AiProtocol {
    Openai,
    Anthropic,
}

/// AI 配置（不含 api_key，key 单独存）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub protocol: AiProtocol,
    pub base_url: String,
    pub model: String,
    /// 0-2 的 temperature，默认 0.7
    #[serde(default = "default_temp")]
    pub temperature: f64,
    /// 最大输出 token，默认 2048
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_temp() -> f64 {
    0.7
}
fn default_max_tokens() -> u32 {
    2048
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            protocol: AiProtocol::Openai,
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o-mini".into(),
            temperature: 0.7,
            max_tokens: 2048,
        }
    }
}

/// 配置文件路径：工作区 .studio/ai_config.json
fn config_file() -> std::path::PathBuf {
    workspace::studio_dir().join("ai_config.json")
}

/// 凭证文件路径：工作区 .studio/ai_key.txt（明文，便携优先；M3 不引入 keyring）
fn key_file() -> std::path::PathBuf {
    workspace::studio_dir().join("ai_key.txt")
}

/// 读取 AI 配置。不存在则返回默认。
pub fn load_config() -> DstResult<AiConfig> {
    let path = config_file();
    if !path.exists() {
        return Ok(AiConfig::default());
    }
    let raw = std::fs::read_to_string(&path)?;
    let cfg: AiConfig = serde_json::from_str(&raw)?;
    Ok(cfg)
}

/// 保存 AI 配置。
pub fn save_config(cfg: &AiConfig) -> DstResult<()> {
    let path = config_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// 读取 API key。
pub fn load_key() -> DstResult<String> {
    let path = key_file();
    if !path.exists() {
        return Err(DstError::Other("未配置 API Key".into()));
    }
    let key = std::fs::read_to_string(&path)?.trim().to_string();
    if key.is_empty() {
        return Err(DstError::Other("API Key 为空".into()));
    }
    Ok(key)
}

/// 保存 API key。
pub fn save_key(key: &str) -> DstResult<()> {
    let path = key_file();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, key.trim())?;
    Ok(())
}

/// 是否已配置（配置 + key 都存在）。
pub fn is_configured() -> bool {
    load_config().is_ok() && load_key().is_ok()
}

/// 聊天消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// 固化的 System Prompt（安全边界，用户不可改）
pub fn system_prompt() -> String {
    r#"你是 DevShellTools Studio 的 PowerShell 命令生成助手。你的任务是根据用户需求生成 PowerShell 5.1 兼容的快捷命令函数。

严格规则（违反则拒绝生成）：
1. 禁止生成 git push --force / --force-with-lease
2. 禁止生成 git reset --hard
3. 禁止生成 git clean -fd / -f（真实删除），只允许 -nd / -ndx 预览
4. Stop-Process 必须配合 -Confirm 或在 SupportsShouldProcess 函数内
5. 禁止 [Environment]::SetEnvironmentVariable(..., "User")，只允许 "Process"
6. 禁止 Remove-Item -Recurse -Force（危险删除）
7. 每个函数必须包含 .SYNOPSIS 和至少一个 .EXAMPLE 注释型帮助
8. 函数名首字母小写为公共导出命令，首字母大写（如 Assert-Xxx）为内部辅助函数

输出格式：
- 把生成的 PowerShell 代码放在 ```powershell 代码块中
- 代码块外可加简短说明
- 代码必须包含 @DST-Category 元数据块（如果是新分类）或可追加到现有分类
- 每个函数用标准 PowerShell 注释帮助块

示例输出：
```powershell
<#!
@DST-Category
Name: docker
Title: Docker
Description: 容器管理快捷命令
Aliases: 容器
@DST-Category-End
#>

function dps {
<#
.SYNOPSIS
列出运行中容器。
.EXAMPLE
dps
#>
    [CmdletBinding()] param()
    Assert-DstCommand "docker"
    & docker ps
}
```
"#.to_string()
}

/// 从 AI 响应文本中提取 ```powershell 代码块
pub fn extract_code_blocks(text: &str) -> Vec<String> {
    let mut blocks = vec![];
    let mut in_block = false;
    let mut current = String::new();
    for line in text.lines() {
        if line.trim_start().starts_with("```powershell") || line.trim_start().starts_with("```ps1") {
            in_block = true;
            current.clear();
            continue;
        }
        if in_block && line.trim() == "```" {
            in_block = false;
            if !current.is_empty() {
                blocks.push(current.clone());
            }
            continue;
        }
        if in_block {
            current.push_str(line);
            current.push('\n');
        }
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_single_block() {
        let text = "这是结果：\n```powershell\nfunction foo { }\n```\n完成";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("function foo"));
    }

    #[test]
    fn extract_multiple_blocks() {
        let text = "```powershell\nfunction a {}\n```\n中间\n```powershell\nfunction b {}\n```";
        let blocks = extract_code_blocks(text);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn no_block() {
        let text = "没有代码块的回答";
        let blocks = extract_code_blocks(text);
        assert!(blocks.is_empty());
    }

    #[test]
    fn default_config() {
        let cfg = AiConfig::default();
        assert_eq!(cfg.protocol, AiProtocol::Openai);
        assert!(!cfg.base_url.is_empty());
        assert!(!cfg.model.is_empty());
    }

    #[test]
    fn system_prompt_has_safety_rules() {
        let p = system_prompt();
        assert!(p.contains("禁止"));
        assert!(p.contains("--force"));
        assert!(p.contains("--hard"));
        assert!(p.contains(".SYNOPSIS"));
    }
}