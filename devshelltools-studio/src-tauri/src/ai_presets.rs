use crate::ai_config::AiProtocol;
use serde::Serialize;

/// AI 提供商预设（端点来自各官方文档）。
#[derive(Debug, Clone, Serialize)]
pub struct AiPreset {
    pub id: String,
    pub name: String,
    /// OpenAI 兼容端点（绝大多数提供商都支持）
    pub openai_base_url: String,
    /// Anthropic 兼容端点（仅部分提供商支持，不支持则为空）
    pub anthropic_base_url: String,
    /// OpenAI 协议默认模型
    pub openai_default_model: String,
    /// Anthropic 协议默认模型
    pub anthropic_default_model: String,
}

/// 前端展示用的简化预设（单协议视角）
#[derive(Debug, Clone, Serialize)]
pub struct AiPresetView {
    pub id: String,
    pub name: String,
    pub protocol: AiProtocol,
    pub base_url: String,
    pub default_model: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiEndpointSuggestion {
    pub base_url: String,
    pub default_model: String,
    pub protocol: AiProtocol,
    pub note: String,
}

/// 全部预设（端点来自官方文档）
pub fn list_presets() -> Vec<AiPreset> {
    vec![
        AiPreset {
            id: "deepseek".into(),
            name: "DeepSeek".into(),
            openai_base_url: "https://api.deepseek.com/v1".into(),
            anthropic_base_url: "https://api.deepseek.com/anthropic".into(),
            openai_default_model: "deepseek-chat".into(),
            anthropic_default_model: "deepseek-chat".into(),
        },
        AiPreset {
            id: "glm".into(),
            name: "智谱 GLM".into(),
            openai_base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "glm-4-flash".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "kimi".into(),
            name: "Kimi（月之暗面）".into(),
            openai_base_url: "https://api.moonshot.cn/v1".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "kimi-k3".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "qwen".into(),
            name: "通义千问（阿里百炼）".into(),
            openai_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "qwen-plus".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "minimax".into(),
            name: "MiniMax".into(),
            openai_base_url: "https://api.minimaxi.com/v1".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "MiniMax-Text-01".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "opencode".into(),
            name: "OpenCode".into(),
            openai_base_url: "https://api.opencode.ai/v1".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "gpt-4o-mini".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "ollama-cloud".into(),
            name: "Ollama Cloud".into(),
            openai_base_url: "https://api.ollama.cloud/v1".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "llama3.2".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "hunyuan".into(),
            name: "腾讯混元".into(),
            openai_base_url: "https://api.hunyuan.cloud.tencent.com/v1".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "hunyuan-turbos-latest".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "mimo".into(),
            name: "小米 MiMo".into(),
            openai_base_url: "https://api.mimo.xiaomi.com/v1".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "mimo-7b".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "volcengine".into(),
            name: "字节火山引擎".into(),
            openai_base_url: "https://ark.cn-beijing.volces.com/api/v3".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "doubao-pro-32k".into(),
            anthropic_default_model: String::new(),
        },
        AiPreset {
            id: "stepfun".into(),
            name: "星辰阶跃（StepFun）".into(),
            openai_base_url: "https://api.stepfun.com/v1".into(),
            anthropic_base_url: String::new(),
            openai_default_model: "step-1-flash".into(),
            anthropic_default_model: String::new(),
        },
    ]
}

/// 前端获取预设列表（扁平化为单协议视角）
pub fn list_preset_views() -> Vec<AiPresetView> {
    list_presets()
        .iter()
        .flat_map(|p| {
            let mut views = vec![AiPresetView {
                id: format!("{}-openai", p.id),
                name: format!("{}（OpenAI 兼容）", p.name),
                protocol: AiProtocol::Openai,
                base_url: p.openai_base_url.clone(),
                default_model: p.openai_default_model.clone(),
            }];
            if !p.anthropic_base_url.is_empty() {
                views.push(AiPresetView {
                    id: format!("{}-anthropic", p.id),
                    name: format!("{}（Anthropic 兼容）", p.name),
                    protocol: AiProtocol::Anthropic,
                    base_url: p.anthropic_base_url.clone(),
                    default_model: p.anthropic_default_model.clone(),
                });
            }
            views
        })
        .collect()
}

/// 根据 base_url 检测所属提供商
fn detect_preset(base_url: &str) -> Option<AiPreset> {
    let u = base_url.to_lowercase();
    list_presets().into_iter().find(|p| {
        let oai = p.openai_base_url.to_lowercase();
        let ant = p.anthropic_base_url.to_lowercase();
        (!oai.is_empty() && u.contains(&domain_of(&oai)))
            || (!ant.is_empty() && u.contains(&domain_of(&ant)))
    })
}

/// 提取 URL 的域名部分用于匹配
fn domain_of(url: &str) -> String {
    let no_scheme = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")).unwrap_or(url);
    let no_trailing = no_scheme.trim_end_matches('/');
    // 取第一段路径之前的部分作为域名
    no_trailing.split('/').next().unwrap_or(no_trailing).to_string()
}

/// 切换协议时，根据当前 base_url 找到对应提供商，返回该协议下的端点。
/// 如果当前提供商不支持目标协议，返回 OpenAI 官方端点作为 fallback。
pub fn suggest_endpoint(protocol: AiProtocol, current_base_url: Option<&str>) -> AiEndpointSuggestion {
    if let Some(url) = current_base_url.filter(|s| !s.trim().is_empty()) {
        if let Some(preset) = detect_preset(url) {
            return match protocol {
                AiProtocol::Openai => {
                    if !preset.openai_base_url.is_empty() {
                        AiEndpointSuggestion {
                            base_url: preset.openai_base_url.clone(),
                            default_model: preset.openai_default_model.clone(),
                            protocol,
                            note: format!("已切换为「{}」OpenAI 兼容端点", preset.name),
                        }
                    } else {
                        AiEndpointSuggestion {
                            base_url: "https://api.openai.com/v1".into(),
                            default_model: "gpt-4o-mini".into(),
                            protocol,
                            note: format!("「{}」不支持 OpenAI 兼容协议，已切换为 OpenAI 官方端点", preset.name),
                        }
                    }
                }
                AiProtocol::Anthropic => {
                    if !preset.anthropic_base_url.is_empty() {
                        AiEndpointSuggestion {
                            base_url: preset.anthropic_base_url.clone(),
                            default_model: preset.anthropic_default_model.clone(),
                            protocol,
                            note: format!("已切换为「{}」Anthropic 兼容端点", preset.name),
                        }
                    } else {
                        AiEndpointSuggestion {
                            base_url: "https://api.anthropic.com/v1".into(),
                            default_model: "claude-3-5-haiku-20241022".into(),
                            protocol,
                            note: format!("「{}」不支持 Anthropic 兼容协议，已切换为 Anthropic 官方端点", preset.name),
                        }
                    }
                }
            };
        }
    }

    // 无法识别提供商，返回协议默认端点
    match protocol {
        AiProtocol::Openai => AiEndpointSuggestion {
            base_url: "https://api.openai.com/v1".into(),
            default_model: "gpt-4o-mini".into(),
            protocol,
            note: "已填充 OpenAI 官方端点".into(),
        },
        AiProtocol::Anthropic => AiEndpointSuggestion {
            base_url: "https://api.anthropic.com/v1".into(),
            default_model: "claude-3-5-haiku-20241022".into(),
            protocol,
            note: "已填充 Anthropic 官方端点".into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deepseek_protocol_switch() {
        // DeepSeek OpenAI → Anthropic：应切到 /anthropic 端点
        let s = suggest_endpoint(AiProtocol::Anthropic, Some("https://api.deepseek.com/v1"));
        assert_eq!(s.base_url, "https://api.deepseek.com/anthropic");
        assert!(s.note.contains("DeepSeek"));
    }

    #[test]
    fn deepseek_anthropic_to_openai() {
        // DeepSeek Anthropic → OpenAI：应切到 /v1 端点
        let s = suggest_endpoint(AiProtocol::Openai, Some("https://api.deepseek.com/anthropic"));
        assert_eq!(s.base_url, "https://api.deepseek.com/v1");
    }

    #[test]
    fn glm_no_anthropic_support() {
        // GLM 不支持 Anthropic，应 fallback 到 Anthropic 官方
        let s = suggest_endpoint(AiProtocol::Anthropic, Some("https://open.bigmodel.cn/api/paas/v4"));
        assert_eq!(s.base_url, "https://api.anthropic.com/v1");
        assert!(s.note.contains("不支持"));
    }

    #[test]
    fn unknown_url_falls_back_to_default() {
        let s = suggest_endpoint(AiProtocol::Openai, Some("https://unknown.example.com/v1"));
        assert_eq!(s.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn all_presets_have_openai_url() {
        let presets = list_presets();
        assert_eq!(presets.len(), 11);
        for p in &presets {
            assert!(!p.openai_base_url.is_empty(), "{} 缺 OpenAI 端点", p.name);
            assert!(!p.openai_default_model.is_empty(), "{} 缺 OpenAI 默认模型", p.name);
        }
    }
}