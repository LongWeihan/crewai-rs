use std::{collections::BTreeMap, fs, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    agent::Agent,
    crew::{Crew, Process},
    error::{CrewError, Result},
    model::ChatModel,
    task::{OutputFormat, Task},
    tool::Tool,
};

#[derive(Default, Clone)]
pub struct RuntimeRegistry {
    models: BTreeMap<String, Arc<dyn ChatModel>>,
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, name: impl Into<String>, model: Arc<dyn ChatModel>) -> Self {
        self.models.insert(name.into(), model);
        self
    }

    pub fn with_tool(mut self, name: impl Into<String>, tool: Arc<dyn Tool>) -> Self {
        self.tools.insert(name.into(), tool);
        self
    }

    pub fn register_model(&mut self, name: impl Into<String>, model: Arc<dyn ChatModel>) {
        self.models.insert(name.into(), model);
    }

    pub fn register_tool(&mut self, name: impl Into<String>, tool: Arc<dyn Tool>) {
        self.tools.insert(name.into(), tool);
    }

    pub fn resolve_model(&self, name: &str) -> Result<Arc<dyn ChatModel>> {
        self.models
            .get(name)
            .cloned()
            .ok_or_else(|| CrewError::MissingModel(name.to_string()))
    }

    pub fn resolve_tool(&self, name: &str) -> Result<Arc<dyn Tool>> {
        self.tools
            .get(name)
            .cloned()
            .ok_or_else(|| CrewError::MissingTool(name.to_string()))
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProcessKind {
    #[default]
    Sequential,
    Hierarchical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewBlueprint {
    pub name: String,
    #[serde(default)]
    pub process: ProcessKind,
    pub agents: Vec<AgentBlueprint>,
    pub tasks: Vec<TaskBlueprint>,
    pub manager: Option<AgentBlueprint>,
}

impl CrewBlueprint {
    pub fn from_yaml_str(value: &str) -> Result<Self> {
        Ok(serde_yaml::from_str(value)?)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_yaml_str(&content)
    }

    pub fn to_yaml_string(&self) -> Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    pub fn build(&self, registry: &RuntimeRegistry) -> Result<Crew> {
        let mut builder = Crew::builder(&self.name);

        for agent in &self.agents {
            builder = builder.agent(agent.build(registry)?);
        }

        for task in &self.tasks {
            builder = builder.task(task.build()?);
        }

        builder = builder.process(match self.process {
            ProcessKind::Sequential => Process::Sequential,
            ProcessKind::Hierarchical => Process::Hierarchical {
                manager: self
                    .manager
                    .as_ref()
                    .ok_or_else(|| {
                        CrewError::InvalidConfig(
                            "hierarchical blueprints require a manager agent".to_string(),
                        )
                    })?
                    .build(registry)?,
            },
        });

        builder.build()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBlueprint {
    pub id: String,
    pub role: String,
    pub goal: String,
    pub backstory: Option<String>,
    pub model: String,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default = "default_iterations")]
    pub max_iterations: usize,
    pub temperature: Option<f32>,
    #[serde(default)]
    pub allow_delegation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBlueprint {
    pub id: String,
    pub description: String,
    pub expected_output: Option<String>,
    pub agent: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    pub context: Option<String>,
    #[serde(default)]
    pub output_format: OutputFormat,
}

impl AgentBlueprint {
    pub fn build(&self, registry: &RuntimeRegistry) -> Result<Agent> {
        let mut builder = Agent::builder(&self.id)
            .role(&self.role)
            .goal(&self.goal)
            .model_ref(registry.resolve_model(&self.model)?)
            .max_iterations(self.max_iterations)
            .allow_delegation(self.allow_delegation);

        if let Some(backstory) = &self.backstory {
            builder = builder.backstory(backstory);
        }

        if let Some(temperature) = self.temperature {
            builder = builder.temperature(temperature);
        }

        for tool in &self.tools {
            builder = builder.tool_ref(registry.resolve_tool(tool)?);
        }

        builder.build()
    }
}

impl TaskBlueprint {
    pub fn build(&self) -> Result<Task> {
        let mut builder = Task::builder(&self.id)
            .description(&self.description)
            .agent(&self.agent)
            .depends_on(self.depends_on.clone())
            .output_format(self.output_format);

        if let Some(expected_output) = &self.expected_output {
            builder = builder.expected_output(expected_output);
        }

        if let Some(context) = &self.context {
            builder = builder.context(context);
        }

        builder.build()
    }
}

fn default_iterations() -> usize {
    4
}
