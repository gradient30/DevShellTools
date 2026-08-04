use crate::ai_config::AiProtocol;
use serde::Serialize;

/// 社区常用的 AI 提供商预设（端点来自官方文档）。
#[derive(Debug, Clone, Serialize)]
pub struct AiPreset {
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

pub fn list_presets() -> Vec<AiPreset> {
    vec![
        AiPreset {
            id: "openai".into(),
            name: "OpenAI".into(),
            protocol: AiProtocol::Openai,
            base_url: "https://api.openai.com/v1".into(),
            default_model: "gpt-4o-mini".into(),
        },
        AiPreset {
            id: "anthropic".into(),
            name: "Anthropic".into(),
            protocol: AiProtocol::Anthropic,
            base_url: "https://api.anthropic.com/v1".into(),
            default_model: "claude-3-5-haiku-20241022".into(),
        },
        AiPreset {
            id: "deepseek".into(),
            name: "DeepSeek（OpenAI 兼容）".into(),
            protocol: AiProtocol::Openai,
            base_url: "https://api.deepseek.com/v1".into(),
            default_model: "deepseek-chat".into(),
        },
        AiPreset {
            id: "moonshot".into(),
            name: "Moonshot / Kimi".into(),
            protocol: AiProtocol::Openai,
            base_url: "https://api.moonshot.cn/v1".into(),
            default_model: "moonshot-v1-8k".into(),
        },
        AiPreset {
            id: "zhipu".into(),
            name: "智谱 GLM".into(),
            protocol: AiProtocol::Openai,
            base_url: "https://open.bigmodel.cn/api/paas/v4".into(),
            default_model: "glm-4-flash".into(),
        },
        AiPreset {
            id: "groq".into(),
            name: "Groq".into(),
            protocol: AiProtocol::Openai,
            base_url: "https://api.groq.com/openai/v1".into(),
            default_model: "llama-3.3-70b-versatile".into(),
        },
        AiPreset {
            id: "openrouter".into(),
            name: "OpenRouter".into(),
            protocol: AiProtocol::Openai,
            base_url: "https://openrouter.ai/api/v1".into(),
            default_model: "openai/gpt-4o-mini".into(),
        },
        AiPreset {
            id: "ollama".into(),
            name: "Ollama（本地）".into(),
            protocol: AiProtocol::Openai,
            base_url: "http://127.0.0.1:11434/v1".into(),
            default_model: "llama3.2".into(),
        },
    ]
}

fn detect_preset_id(base_url: &str) -> Option<&'static str> {
    let u = base_url.to_lowercase();
    if u.contains("api.openai.com") {
        Some("openai")
    } else if u.contains("api.anthropic.com") {
        Some("anthropic")
    } else if u.contains("api.deepseek.com") {
        Some("deepseek")
    } else if u.contains("moonshot.cn") {
        Some("moonshot")
    } else if u.contains("bigmodel.cn") {
        Some("zhipu")
    } else if u.contains("groq.com") {
        Some("groq")
    } else if u.contains("openrouter.ai") {
        Some("openrouter")
    } else if u.contains("127.0.0.1:11434") || u.contains("localhost:11434") {
        Some("ollama")
    } else {
        None
    }
}

/// 切换协议或提供商时，给出匹配的 Base URL 与默认模型。
pub fn suggest_endpoint(protocol: AiProtocol, current_base_url: Option<&str>) -> AiEndpointSuggestion {
    let presets = list_presets();
    if let Some(url) = current_base_url.filter(|s| !s.trim().is_empty()) {
        if let Some(pid) = detect_preset_id(url) {
            if let Some(p) = presets.iter().find(|x| x.id == pid) {
                if p.protocol == protocol {
                    return AiEndpointSuggestion {
                        base_url: p.base_url.clone(),
                        default_model: p.default_model.clone(),
                        protocol,
                        note: format!("已匹配提供商「{}」", p.name),
                    };
                }
                // URL 与所选协议不匹配：按目标协议填充官方/默认端点
                let target = presets
                    .iter()
                    .find(|x| x.protocol == protocol)
                    .unwrap();
                return AiEndpointSuggestion {
                    base_url: target.base_url.clone(),
                    default_model: target.default_model.clone(),
                    protocol,
                    note: format!(
                        "「{}」与所选协议不匹配，已切换为「{}」端点",
                        p.name, target.name
                    ),
                };
            }
        }
    }

    let fallback = presets
        .iter()
        .find(|p| p.protocol == protocol)
        .or_else(|| presets.first())
        .unwrap();
    AiEndpointSuggestion {
        base_url: fallback.base_url.clone(),
        default_model: fallback.default_model.clone(),
        protocol: fallback.protocol,
        note: format!("已填充「{}」默认端点", fallback.name),
    }
}

pub fn anthropic_model_catalog() -> Vec<String> {
    vec![
        "claude-3-5-haiku-20241022".into(),
        "claude-3-5-sonnet-20241022".into(),
        "claude-3-opus-20240229".into(),
        "claude-3-haiku-20240307".into(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_config::AiProtocol;

    #[test]
    fn suggest_openai_from_anthropic_url() {
        let s = suggest_endpoint(
            AiProtocol::Openai,
            Some("https://api.anthropic.com/v1"),
        );
        assert_eq!(s.protocol, AiProtocol::Openai);
        assert!(s.base_url.contains("openai.com"));
    }
}
