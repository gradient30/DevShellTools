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

// ============ OpenAI 协议 ============

#[derive(Serialize)]
struct OpenaiRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    max_tokens: u32,
    stream: bool,
}

#[derive(Deserialize)]
struct OpenaiStreamChunk {
    choices: Vec<OpenaiStreamChoice>,
}

#[derive(Deserialize)]
struct OpenaiStreamChoice {
    delta: OpenaiStreamDelta,
}

#[derive(Deserialize)]
struct OpenaiStreamDelta {
    content: Option<String>,
}

async fn chat_openai(
    config: &AiConfig,
    api_key: &str,
    messages: Vec<ChatMessage>,
) -> DstResult<Vec<StreamEvent>> {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let body = OpenaiRequest {
        model: config.model.clone(),
        messages,
        temperature: config.temperature,
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

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| DstError::Http(e))?;
        let text = String::from_utf8_lossy(&chunk);
        buffer.push_str(&text);

        // 解析 SSE：每行 "data: {...}" 或 "data: [DONE]"
        while let Some(pos) = buffer.find('\n') {
            let line: String = buffer.drain(..=pos).collect();
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(data) = line.strip_prefix("data: ") {
                if data.trim() == "[DONE]" {
                    events.push(StreamEvent {
                        kind: "done".into(),
                        content: String::new(),
                    });
                    return Ok(events);
                }
                if let Ok(chunk) = serde_json::from_str::<OpenaiStreamChunk>(data) {
                    if let Some(content) = chunk
                        .choices
                        .first()
                        .and_then(|c| c.delta.content.as_ref())
                    {
                        events.push(StreamEvent {
                            kind: "delta".into(),
                            content: content.clone(),
                        });
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

// ============ Anthropic 协议 ============

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<ChatMessage>,
    system: String,
    temperature: f64,
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
        temperature: config.temperature,
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

/// 拉取可用模型列表（OpenAI 兼容走 /models，Anthropic 返回常用目录）。
pub async fn list_models(config: &AiConfig, api_key: &str) -> DstResult<Vec<String>> {
    match config.protocol {
        AiProtocol::Openai => list_openai_models(config, api_key).await,
        AiProtocol::Anthropic => Ok(crate::ai_presets::anthropic_model_catalog()),
    }
}

#[derive(Deserialize)]
struct OpenaiModelsResponse {
    data: Vec<OpenaiModelItem>,
}

#[derive(Deserialize)]
struct OpenaiModelItem {
    id: String,
}

async fn list_openai_models(config: &AiConfig, api_key: &str) -> DstResult<Vec<String>> {
    let url = format!("{}/models", config.base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(DstError::Http)?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(DstError::Other(format!(
            "拉取模型失败 {status}：{text}（请确认协议为 OpenAI 兼容且 Base URL 正确，如 DeepSeek 应为 https://api.deepseek.com/v1）"
        )));
    }

    let parsed: OpenaiModelsResponse = resp.json().await.map_err(DstError::Http)?;
    let mut ids: Vec<String> = parsed.data.into_iter().map(|m| m.id).collect();
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
}