use std::{collections::BTreeMap, future::Future, pin::Pin, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::Result;

type ToolFuture = Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send>>;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolInput {
    pub value: String,
    pub task_id: String,
    pub agent_id: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl ToolInput {
    pub fn new(
        value: impl Into<String>,
        task_id: impl Into<String>,
        agent_id: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),
            task_id: task_id.into(),
            agent_id: agent_id.into(),
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolOutput {
    pub content: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl ToolOutput {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallRecord {
    pub tool: String,
    pub input: String,
    pub output: String,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn call(&self, input: ToolInput) -> Result<ToolOutput>;
}

#[derive(Clone)]
pub struct FnTool {
    name: String,
    description: String,
    handler: Arc<dyn Fn(ToolInput) -> ToolFuture + Send + Sync>,
}

impl std::fmt::Debug for FnTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FnTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .finish()
    }
}

impl FnTool {
    pub fn new<F, Fut>(name: impl Into<String>, description: impl Into<String>, handler: F) -> Self
    where
        F: Fn(ToolInput) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolOutput>> + Send + 'static,
    {
        Self {
            name: name.into(),
            description: description.into(),
            handler: Arc::new(move |input| Box::pin(handler(input))),
        }
    }
}

#[async_trait]
impl Tool for FnTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn call(&self, input: ToolInput) -> Result<ToolOutput> {
        (self.handler)(input).await
    }
}
