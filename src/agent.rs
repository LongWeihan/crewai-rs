use std::sync::Arc;

use crate::{
    error::{CrewError, Result},
    model::ChatModel,
    tool::Tool,
};

#[derive(Clone)]
pub struct Agent {
    id: String,
    role: String,
    goal: String,
    backstory: Option<String>,
    model: Arc<dyn ChatModel>,
    tools: Vec<Arc<dyn Tool>>,
    max_iterations: usize,
    temperature: Option<f32>,
    allow_delegation: bool,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tool_names = self
            .tools
            .iter()
            .map(|tool| tool.name())
            .collect::<Vec<_>>();
        f.debug_struct("Agent")
            .field("id", &self.id)
            .field("role", &self.role)
            .field("goal", &self.goal)
            .field("backstory", &self.backstory)
            .field("model", &self.model.name())
            .field("tools", &tool_names)
            .field("max_iterations", &self.max_iterations)
            .field("temperature", &self.temperature)
            .field("allow_delegation", &self.allow_delegation)
            .finish()
    }
}

impl Agent {
    pub fn builder(id: impl Into<String>) -> AgentBuilder {
        AgentBuilder {
            id: id.into(),
            role: None,
            goal: None,
            backstory: None,
            model: None,
            tools: Vec::new(),
            max_iterations: 4,
            temperature: None,
            allow_delegation: false,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn role(&self) -> &str {
        &self.role
    }

    pub fn goal(&self) -> &str {
        &self.goal
    }

    pub fn backstory(&self) -> Option<&str> {
        self.backstory.as_deref()
    }

    pub fn model(&self) -> &Arc<dyn ChatModel> {
        &self.model
    }

    pub fn tools(&self) -> &[Arc<dyn Tool>] {
        &self.tools
    }

    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    pub fn temperature(&self) -> Option<f32> {
        self.temperature
    }

    pub fn allow_delegation(&self) -> bool {
        self.allow_delegation
    }

    pub fn find_tool(&self, tool_name: &str) -> Option<Arc<dyn Tool>> {
        self.tools
            .iter()
            .find(|tool| tool.name() == tool_name)
            .cloned()
    }
}

#[derive(Clone)]
pub struct AgentBuilder {
    id: String,
    role: Option<String>,
    goal: Option<String>,
    backstory: Option<String>,
    model: Option<Arc<dyn ChatModel>>,
    tools: Vec<Arc<dyn Tool>>,
    max_iterations: usize,
    temperature: Option<f32>,
    allow_delegation: bool,
}

impl AgentBuilder {
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn goal(mut self, goal: impl Into<String>) -> Self {
        self.goal = Some(goal.into());
        self
    }

    pub fn backstory(mut self, backstory: impl Into<String>) -> Self {
        self.backstory = Some(backstory.into());
        self
    }

    pub fn model<M>(mut self, model: M) -> Self
    where
        M: ChatModel + 'static,
    {
        self.model = Some(Arc::new(model));
        self
    }

    pub fn model_ref(mut self, model: Arc<dyn ChatModel>) -> Self {
        self.model = Some(model);
        self
    }

    pub fn tool<T>(mut self, tool: T) -> Self
    where
        T: Tool + 'static,
    {
        self.tools.push(Arc::new(tool));
        self
    }

    pub fn tool_ref(mut self, tool: Arc<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn allow_delegation(mut self, allow_delegation: bool) -> Self {
        self.allow_delegation = allow_delegation;
        self
    }

    pub fn build(self) -> Result<Agent> {
        let role = self
            .role
            .ok_or_else(|| CrewError::InvalidConfig("agent role is required".to_string()))?;
        let goal = self
            .goal
            .ok_or_else(|| CrewError::InvalidConfig("agent goal is required".to_string()))?;
        let model = self.model.ok_or_else(|| {
            CrewError::InvalidConfig(format!("agent `{}` requires a model", self.id))
        })?;

        if self.id.trim().is_empty() {
            return Err(CrewError::InvalidConfig(
                "agent id cannot be empty".to_string(),
            ));
        }

        if self.max_iterations == 0 {
            return Err(CrewError::InvalidConfig(format!(
                "agent `{}` must allow at least one iteration",
                self.id
            )));
        }

        Ok(Agent {
            id: self.id,
            role,
            goal,
            backstory: self.backstory,
            model,
            tools: self.tools,
            max_iterations: self.max_iterations,
            temperature: self.temperature,
            allow_delegation: self.allow_delegation,
        })
    }
}
