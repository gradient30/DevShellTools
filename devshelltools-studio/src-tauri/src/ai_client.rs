use crate::ai_config::{self, AiConfig, AiProtocol, ChatMessage};
use crate::error::{DstError, DstResult};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

/// 一次流式请求的输入
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
}

/// 流式产生的事件
#[derive(Debug, Clone, Serialize)]
pub struct StreamEvent {
    pub kind: String, // "delta" | "done" | "error"
    pub content: String,
}

/// 构造一次完整 chat 请求并启动流式。
/// 返回一个 Vec<StreamEvent>（M3 阶段先收集后返回，UI 异步展示；
/// 真正逐 token 推送需 Tauri event，M3.5 优化）。
pub async fn chat_stream(
    config: &AiConfig,
    api_key: &str,
    messages: Vec<ChatMessage>,
) -> DstResult<Vec<StreamEvent>> {
    // 注入 system prompt 作为首条消息
    let mut full_messages = vec![ChatMessage {
        role: "system".into(),
        content: ai_config::system_prompt(),
    }];
    full_messages.extend(messages);

    match config.protocol {
        AiProtocol::Openai => chat_openai(config, api_key, full_messages).await,
        AiProtocol::Anthropic => chat_anthropic(config, api_key, full_messages).await,
    }
}

// ============ 端点归一化 ============

/// 纠正已知失效/错误的 OpenAI 兼容 Base URL。
/// 例如旧预设 `https://api.ollama.cloud/v1` 现已全局 503，应改用官方 `https://ollama.com/v1`。
fn normalize_openai_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    let lower = trimmed.to_lowercase();
    if lower.contains("api.ollama.cloud") {
        return "https://ollama.com/v1".into();
    }
    trimmed.to_string()
}

// ============ 采样参数策略 ============

/// 部分模型对 temperature 有硬性限制：
/// - Kimi K2/K3 系列：固定采样，官方要求不传（传非 1 会 400）
/// - OpenAI o1/o3/o4 推理系列：不支持 temperature
/// - deepseek-reasoner 等：历史上曾拒绝该参数，省略更安全
/// 返回 None 表示请求体中省略该字段。
fn temperature_for_request(model: &str, configured: f64) -> Option<f64> {
    let m = model.to_lowercase();
    if m.starts_with("kimi-k") {
        return None;
    }
    if m.starts_with("o1")
        || m.starts_with("o3")
        || m.starts_with("o4-")
        || m == "o4"
    {
        return None;
    }
    if m.contains("reasoner") {
        return None;
    }
    Some(configured)
}

// ============ OpenAI 协议 ============

#[derive(Serialize)]
struct OpenaiRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Deserialize)]
struct OpenaiStreamChunk {
    #[serde(default)]
    choices: Vec<OpenaiStreamChoice>,
}

#[derive(Deserialize)]
struct OpenaiStreamChoice {
    delta: Option<OpenaiStreamDelta>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenaiStreamDelta {
    content: Option<String>,
    /// DeepSeek / 部分思考模型：思考过程走此字段，正文仍在 content
    reasoning_content: Option<String>,
}

async fn chat_openai(
    config: &AiConfig,
    api_key: &str,
    messages: Vec<ChatMessage>,
) -> DstResult<Vec<StreamEvent>> {
    let base = normalize_openai_base_url(&config.base_url);
    let url = format!("{base}/chat/completions");
    let body = OpenaiRequest {
        model: config.model.clone(),
        messages,
        temperature: temperature_for_request(&config.model, config.temperature),
        max_tokens: config.max_tokens,
        stream: true,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| DstError::Http(e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(DstError::Other(format!("OpenAI 返回 {status}: {text}")));
    }

    let mut events = vec![];
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();
    let mut saw_reasoning = false;
    let mut finish_reason = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| DstError::Http(e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // 解析 SSE：每行 "data: {...}" 或 "data: [DONE]"（兼容 data: 后无空格）
        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            let line = line.trim().trim_end_matches('\r');
            if line.is_empty() {
                continue;
            }
            let Some(data) = line
                .strip_prefix("data: ")
                .or_else(|| line.strip_prefix("data:"))
            else {
                continue;
            };
            let data = data.trim();
            if data == "[DONE]" {
                events.push(StreamEvent {
                    kind: "done".into(),
                    content: finish_reason.clone(),
                });
                return finalize_openai_events(events, saw_reasoning);
            }
            if let Ok(chunk) = serde_json::from_str::<OpenaiStreamChunk>(data) {
                if let Some(choice) = chunk.choices.first() {
                    if let Some(fr) = choice.finish_reason.as_ref() {
                        if !fr.is_empty() {
                            finish_reason = fr.clone();
                        }
                    }
                    if let Some(delta) = choice.delta.as_ref() {
                        if let Some(r) = delta.reasoning_content.as_ref() {
                            if !r.is_empty() {
                                saw_reasoning = true;
                                events.push(StreamEvent {
                                    kind: "reasoning".into(),
                                    content: r.clone(),
                                });
                            }
                        }
                        if let Some(content) = delta.content.as_ref() {
                            if !content.is_empty() {
                                events.push(StreamEvent {
                                    kind: "delta".into(),
                                    content: content.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    events.push(StreamEvent {
        kind: "done".into(),
        content: finish_reason,
    });
    finalize_openai_events(events, saw_reasoning)
}

/// 思考模型若把 token 耗在 reasoning 上，正文可能为空——直接报明确错误，避免前端“假中断”。
fn finalize_openai_events(
    events: Vec<StreamEvent>,
    saw_reasoning: bool,
) -> DstResult<Vec<StreamEvent>> {
    let content_len: usize = events
        .iter()
        .filter(|e| e.kind == "delta")
        .map(|e| e.content.len())
        .sum();
    if content_len == 0 && saw_reasoning {
        let preview: String = events
            .iter()
            .filter(|e| e.kind == "reasoning")
            .map(|e| e.content.as_str())
            .collect::<String>()
            .chars()
            .take(120)
            .collect();
        let finish = events
            .iter()
            .rev()
            .find(|e| e.kind == "done")
            .map(|e| e.content.as_str())
            .unwrap_or("");
        return Err(DstError::Other(format!(
            "模型只返回了思考过程、未产出正文（常见原因：思考模式占满 max_tokens）。\
请将配置中的 max_tokens 调到 8192 以上，或关闭思考模式 / 改用非思考模型。\
finish_reason={finish}；思考预览：{preview}…"
        )));
    }
    Ok(events)
}

// ============ Anthropic 协议 ============

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<ChatMessage>,
    system: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    max_tokens: u32,
    stream: bool,
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    delta: Option<AnthropicDelta>,
}

#[derive(Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: Option<String>,
    text: Option<String>,
}

async fn chat_anthropic(
    config: &AiConfig,
    api_key: &str,
    messages: Vec<ChatMessage>,
) -> DstResult<Vec<StreamEvent>> {
    let url = format!("{}/messages", config.base_url.trim_end_matches('/'));

    // Anthropic 的 system 是顶层字段，不在 messages 里
    let system = messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let user_messages: Vec<ChatMessage> = messages
        .into_iter()
        .filter(|m| m.role != "system")
        .collect();

    let body = AnthropicRequest {
        model: config.model.clone(),
        messages: user_messages,
        system,
        temperature: temperature_for_request(&config.model, config.temperature),
        max_tokens: config.max_tokens,
        stream: true,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| DstError::Http(e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(DstError::Other(format!("Anthropic 返回 {status}: {text}")));
    }

    let mut events = vec![];
    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| DstError::Http(e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                    match event.event_type.as_str() {
                        "content_block_delta" => {
                            if let Some(delta) = event.delta {
                                if let Some(text) = delta.text {
                                    events.push(StreamEvent {
                                        kind: "delta".into(),
                                        content: text,
                                    });
                                }
                            }
                        }
                        "message_stop" => {
                            events.push(StreamEvent {
                                kind: "done".into(),
                                content: String::new(),
                            });
                            return Ok(events);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    events.push(StreamEvent {
        kind: "done".into(),
        content: String::new(),
    });
    Ok(events)
}

#[derive(Deserialize)]
struct OpenaiModelsResponse {
    data: Vec<OpenaiModelItem>,
}

#[derive(Deserialize)]
struct OpenaiModelItem {
    id: String,
}

/// 拉取可用模型列表。
/// OpenAI 和 Anthropic 协议都走 GET /v1/models 真实拉取（DeepSeek 等 OpenAI 兼容服务也支持此端点）。
/// Anthropic 官方 API 也可用 /v1/models（需 x-api-key 头）。
/// Ollama（含 Cloud）在 OpenAI `/models` 失败时回退原生 `/api/tags`。
pub async fn list_models(config: &AiConfig, api_key: &str) -> DstResult<Vec<String>> {
    let base = match config.protocol {
        AiProtocol::Openai => normalize_openai_base_url(&config.base_url),
        AiProtocol::Anthropic => config.base_url.trim().trim_end_matches('/').to_string(),
    };
    let url = format!("{base}/models");
    let client = reqwest::Client::new();
    let req = match config.protocol {
        AiProtocol::Openai => client.get(&url).bearer_auth(api_key),
        AiProtocol::Anthropic => client
            .get(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01"),
    };
    let resp = req.send().await.map_err(DstError::Http)?;

    if resp.status().is_success() {
        let parsed: OpenaiModelsResponse = resp.json().await.map_err(DstError::Http)?;
        let mut ids: Vec<String> = parsed.data.into_iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        if ids.is_empty() {
            return Err(DstError::Other("模型列表为空".into()));
        }
        return Ok(ids);
    }

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    // Ollama Cloud / 本地：回退原生 tags 接口
    if matches!(config.protocol, AiProtocol::Openai) && is_ollama_base(&base) {
        if let Ok(ids) = list_ollama_tags(&client, &base, api_key).await {
            return Ok(ids);
        }
    }

    Err(DstError::Other(format!("拉取模型失败 {status}：{text}")))
}

fn is_ollama_base(base_url: &str) -> bool {
    let u = base_url.to_lowercase();
    u.contains("ollama.com") || u.contains("ollama.cloud") || u.contains(":11434")
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaTagItem>,
}

#[derive(Deserialize)]
struct OllamaTagItem {
    name: String,
}

async fn list_ollama_tags(
    client: &reqwest::Client,
    openai_base: &str,
    api_key: &str,
) -> DstResult<Vec<String>> {
    let tags_url = if let Some(root) = openai_base
        .trim_end_matches('/')
        .strip_suffix("/v1")
    {
        format!("{root}/api/tags")
    } else {
        format!("{}/api/tags", openai_base.trim_end_matches('/'))
    };
    let mut req = client.get(&tags_url);
    if !api_key.trim().is_empty() {
        req = req.bearer_auth(api_key);
    }
    let resp = req.send().await.map_err(DstError::Http)?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(DstError::Other(format!(
            "Ollama /api/tags 失败 {status}：{text}"
        )));
    }
    let parsed: OllamaTagsResponse = resp.json().await.map_err(DstError::Http)?;
    let mut ids: Vec<String> = parsed.models.into_iter().map(|m| m.name).collect();
    ids.sort();
    ids.dedup();
    if ids.is_empty() {
        return Err(DstError::Other("模型列表为空".into()));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_event_serialize() {
        let e = StreamEvent {
            kind: "delta".into(),
            content: "hi".into(),
        };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains("delta"));
        assert!(json.contains("hi"));
    }

    #[test]
    fn kimi_k_models_omit_temperature() {
        assert_eq!(temperature_for_request("kimi-k3", 0.7), None);
        assert_eq!(temperature_for_request("kimi-k2.6", 0.7), None);
        assert_eq!(temperature_for_request("Kimi-K2.5", 1.0), None);
    }

    #[test]
    fn normal_models_keep_temperature() {
        assert_eq!(temperature_for_request("deepseek-chat", 0.7), Some(0.7));
        assert_eq!(temperature_for_request("glm-4-flash", 0.5), Some(0.5));
        assert_eq!(temperature_for_request("qwen-plus", 0.7), Some(0.7));
        assert_eq!(
            temperature_for_request("ernie-4.0-turbo-8k", 0.7),
            Some(0.7)
        );
        assert_eq!(
            temperature_for_request("moonshot-v1-8k", 0.3),
            Some(0.3)
        );
    }

    #[test]
    fn reasoning_models_omit_temperature() {
        assert_eq!(temperature_for_request("o1-mini", 0.7), None);
        assert_eq!(temperature_for_request("o3-mini", 0.7), None);
        assert_eq!(temperature_for_request("deepseek-reasoner", 0.7), None);
    }

    #[test]
    fn openai_request_omits_temperature_when_none() {
        let body = OpenaiRequest {
            model: "kimi-k3".into(),
            messages: vec![],
            temperature: None,
            max_tokens: 1024,
            stream: true,
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(!json.contains("temperature"));
    }

    #[test]
    fn normalize_dead_ollama_cloud_host() {
        assert_eq!(
            normalize_openai_base_url("https://api.ollama.cloud/v1"),
            "https://ollama.com/v1"
        );
        assert_eq!(
            normalize_openai_base_url("https://ollama.com/v1/"),
            "https://ollama.com/v1"
        );
        assert_eq!(
            normalize_openai_base_url("https://api.deepseek.com/v1"),
            "https://api.deepseek.com/v1"
        );
    }

    #[test]
    fn reasoning_only_stream_errors_clearly() {
        let events = vec![
            StreamEvent {
                kind: "reasoning".into(),
                content: "先分析命令安全性".into(),
            },
            StreamEvent {
                kind: "done".into(),
                content: "length".into(),
            },
        ];
        let err = finalize_openai_events(events, true).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("思考过程"), "{msg}");
        assert!(msg.contains("max_tokens"), "{msg}");
    }

    #[test]
    fn content_after_reasoning_ok() {
        let events = vec![
            StreamEvent {
                kind: "reasoning".into(),
                content: "think".into(),
            },
            StreamEvent {
                kind: "delta".into(),
                content: "1. 问题检查".into(),
            },
            StreamEvent {
                kind: "done".into(),
                content: "stop".into(),
            },
        ];
        assert!(finalize_openai_events(events, true).is_ok());
    }
}