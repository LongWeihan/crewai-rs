use std::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::{CrewError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ModelMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl std::ops::AddAssign for TokenUsage {
    fn add_assign(&mut self, rhs: Self) {
        self.prompt_tokens += rhs.prompt_tokens;
        self.completion_tokens += rhs.completion_tokens;
        self.total_tokens += rhs.total_tokens;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequest {
    pub messages: Vec<ModelMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl ModelRequest {
    pub fn new(messages: Vec<ModelMessage>) -> Self {
        Self {
            messages,
            temperature: None,
            max_tokens: None,
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub usage: Option<TokenUsage>,
    pub raw: Option<Value>,
}

impl ModelResponse {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            usage: None,
            raw: None,
        }
    }
}

#[async_trait]
pub trait ChatModel: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, request: ModelRequest) -> Result<ModelResponse>;
}

#[derive(Clone)]
pub struct MockChatModel {
    name: String,
    responses: Arc<Mutex<VecDeque<ModelResponse>>>,
    requests: Arc<Mutex<Vec<ModelRequest>>>,
}

impl std::fmt::Debug for MockChatModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockChatModel")
            .field("name", &self.name)
            .finish()
    }
}

impl MockChatModel {
    pub fn new(name: impl Into<String>, responses: Vec<ModelResponse>) -> Self {
        Self {
            name: name.into(),
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn from_strings<I, S>(name: impl Into<String>, responses: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::new(
            name,
            responses
                .into_iter()
                .map(ModelResponse::text)
                .collect::<Vec<_>>(),
        )
    }

    pub fn requests(&self) -> Vec<ModelRequest> {
        self.requests
            .lock()
            .expect("mock model request lock poisoned")
            .clone()
    }
}

#[async_trait]
impl ChatModel for MockChatModel {
    fn name(&self) -> &str {
        &self.name
    }

    async fn complete(&self, request: ModelRequest) -> Result<ModelResponse> {
        self.requests
            .lock()
            .expect("mock model request lock poisoned")
            .push(request);

        self.responses
            .lock()
            .expect("mock model response lock poisoned")
            .pop_front()
            .ok_or_else(|| {
                CrewError::Model(format!("mock model `{}` ran out of responses", self.name))
            })
    }
}

#[derive(Clone)]
pub struct OpenAIChatModel {
    model: String,
    api_key: String,
    base_url: String,
    default_temperature: Option<f32>,
    client: Client,
}

impl std::fmt::Debug for OpenAIChatModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenAIChatModel")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("default_temperature", &self.default_temperature)
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct OpenAIChatModelBuilder {
    model: String,
    api_key: String,
    base_url: String,
    default_temperature: Option<f32>,
    timeout: Duration,
}

impl OpenAIChatModel {
    pub fn new(model: impl Into<String>, api_key: impl Into<String>) -> Result<Self> {
        Self::builder(model, api_key).build()
    }

    pub fn builder(model: impl Into<String>, api_key: impl Into<String>) -> OpenAIChatModelBuilder {
        OpenAIChatModelBuilder {
            model: model.into(),
            api_key: api_key.into(),
            base_url: "https://api.openai.com".to_string(),
            default_temperature: None,
            timeout: Duration::from_secs(60),
        }
    }
}

impl OpenAIChatModelBuilder {
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.default_temperature = Some(temperature);
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> Result<OpenAIChatModel> {
        if self.model.trim().is_empty() {
            return Err(CrewError::InvalidConfig(
                "openai model name cannot be empty".to_string(),
            ));
        }

        if self.api_key.trim().is_empty() {
            return Err(CrewError::InvalidConfig(
                "openai api key cannot be empty".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(self.timeout)
            .user_agent("crewai-rs/0.1.0")
            .build()?;

        Ok(OpenAIChatModel {
            model: self.model,
            api_key: self.api_key,
            base_url: self.base_url.trim_end_matches('/').to_string(),
            default_temperature: self.default_temperature,
            client,
        })
    }
}

#[async_trait]
impl ChatModel for OpenAIChatModel {
    fn name(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: ModelRequest) -> Result<ModelResponse> {
        let payload = json!({
            "model": self.model,
            "messages": request
                .messages
                .iter()
                .map(|message| {
                    json!({
                        "role": message.role.as_str(),
                        "content": message.content,
                    })
                })
                .collect::<Vec<_>>(),
            "temperature": request.temperature.or(self.default_temperature),
            "max_tokens": request.max_tokens,
        });

        let raw = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json::<Value>()
            .await?;

        let content = extract_openai_content(&raw).ok_or_else(|| {
            CrewError::Model("openai response did not include assistant content".to_string())
        })?;

        Ok(ModelResponse {
            content,
            usage: parse_usage(&raw),
            raw: Some(raw),
        })
    }
}

fn parse_usage(raw: &Value) -> Option<TokenUsage> {
    let usage = raw.get("usage")?;
    Some(TokenUsage {
        prompt_tokens: usage.get("prompt_tokens")?.as_u64()? as u32,
        completion_tokens: usage.get("completion_tokens")?.as_u64()? as u32,
        total_tokens: usage.get("total_tokens")?.as_u64()? as u32,
    })
}

fn extract_openai_content(raw: &Value) -> Option<String> {
    let content = raw
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?;

    match content {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let mut combined = String::new();
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    if !combined.is_empty() {
                        combined.push('\n');
                    }
                    combined.push_str(text);
                }
            }

            if combined.is_empty() {
                None
            } else {
                Some(combined)
            }
        }
        _ => None,
    }
}
