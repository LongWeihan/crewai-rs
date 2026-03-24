use serde::{Deserialize, Serialize};

use crate::error::{CrewError, Result};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Text,
    #[default]
    Markdown,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) expected_output: Option<String>,
    pub(crate) agent: String,
    #[serde(default)]
    pub(crate) depends_on: Vec<String>,
    pub(crate) context: Option<String>,
    #[serde(default)]
    pub(crate) output_format: OutputFormat,
}

impl Task {
    pub fn builder(id: impl Into<String>) -> TaskBuilder {
        TaskBuilder {
            id: id.into(),
            description: None,
            expected_output: None,
            agent: None,
            depends_on: Vec::new(),
            context: None,
            output_format: OutputFormat::Markdown,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn expected_output(&self) -> Option<&str> {
        self.expected_output.as_deref()
    }

    pub fn agent(&self) -> &str {
        &self.agent
    }

    pub fn depends_on(&self) -> &[String] {
        &self.depends_on
    }

    pub fn context(&self) -> Option<&str> {
        self.context.as_deref()
    }

    pub fn output_format(&self) -> OutputFormat {
        self.output_format
    }
}

#[derive(Debug, Clone)]
pub struct TaskBuilder {
    id: String,
    description: Option<String>,
    expected_output: Option<String>,
    agent: Option<String>,
    depends_on: Vec<String>,
    context: Option<String>,
    output_format: OutputFormat,
}

impl TaskBuilder {
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn expected_output(mut self, expected_output: impl Into<String>) -> Self {
        self.expected_output = Some(expected_output.into());
        self
    }

    pub fn agent(mut self, agent: impl Into<String>) -> Self {
        self.agent = Some(agent.into());
        self
    }

    pub fn depends_on<I, S>(mut self, depends_on: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.depends_on = depends_on.into_iter().map(Into::into).collect();
        self
    }

    pub fn context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn output_format(mut self, output_format: OutputFormat) -> Self {
        self.output_format = output_format;
        self
    }

    pub fn build(self) -> Result<Task> {
        if self.id.trim().is_empty() {
            return Err(CrewError::InvalidConfig(
                "task id cannot be empty".to_string(),
            ));
        }

        let description = self.description.ok_or_else(|| {
            CrewError::InvalidConfig(format!("task `{}` requires a description", self.id))
        })?;

        let agent = self.agent.ok_or_else(|| {
            CrewError::InvalidConfig(format!("task `{}` requires an agent", self.id))
        })?;

        Ok(Task {
            id: self.id,
            description,
            expected_output: self.expected_output,
            agent,
            depends_on: self.depends_on,
            context: self.context,
            output_format: self.output_format,
        })
    }
}
