use std::collections::{BTreeMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::{
    agent::Agent,
    error::{CrewError, Result},
    model::{MessageRole, ModelMessage, ModelRequest, TokenUsage},
    task::Task,
    tool::{ToolCallRecord, ToolInput},
};

#[derive(Clone, Default)]
pub enum Process {
    #[default]
    Sequential,
    Hierarchical {
        manager: Agent,
    },
}

impl std::fmt::Debug for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sequential => write!(f, "Sequential"),
            Self::Hierarchical { manager } => f
                .debug_struct("Hierarchical")
                .field("manager", &manager.id())
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KickoffInput {
    pub goal: String,
    #[serde(default)]
    pub context: BTreeMap<String, String>,
}

impl KickoffInput {
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            goal: goal.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

impl From<&str> for KickoffInput {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for KickoffInput {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutcome {
    pub task_id: String,
    pub agent_id: String,
    pub output: String,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRecord>,
    pub manager_brief: Option<String>,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptEventKind {
    TaskStarted,
    ManagerBriefed,
    ToolCalled,
    TaskCompleted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEvent {
    pub kind: TranscriptEventKind,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrewExecution {
    pub crew_name: String,
    pub kickoff: KickoffInput,
    pub final_output: String,
    pub tasks: Vec<TaskOutcome>,
    pub transcript: Vec<TranscriptEvent>,
    pub usage: TokenUsage,
}

impl CrewExecution {
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# {}\n\n", self.crew_name));
        out.push_str("## Goal\n\n");
        out.push_str(&self.kickoff.goal);
        out.push_str("\n\n## Final Output\n\n");
        out.push_str(&self.final_output);
        out.push_str("\n\n## Tasks\n\n");

        for task in &self.tasks {
            out.push_str(&format!(
                "### {} ({})\n\n{}\n\n",
                task.task_id, task.agent_id, task.output
            ));

            if !task.tool_calls.is_empty() {
                out.push_str("Tool calls:\n");
                for call in &task.tool_calls {
                    out.push_str(&format!(
                        "- `{}` input: `{}`\n- output: {}\n",
                        call.tool, call.input, call.output
                    ));
                }
                out.push('\n');
            }
        }

        out
    }
}

#[derive(Clone)]
pub struct Crew {
    name: String,
    agents: BTreeMap<String, Agent>,
    tasks: Vec<Task>,
    process: Process,
}

impl std::fmt::Debug for Crew {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Crew")
            .field("name", &self.name)
            .field("agents", &self.agents.keys().collect::<Vec<_>>())
            .field(
                "tasks",
                &self.tasks.iter().map(|task| task.id()).collect::<Vec<_>>(),
            )
            .field("process", &self.process)
            .finish()
    }
}

impl Crew {
    pub fn builder(name: impl Into<String>) -> CrewBuilder {
        CrewBuilder {
            name: name.into(),
            agents: BTreeMap::new(),
            tasks: Vec::new(),
            process: Process::Sequential,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn agents(&self) -> &BTreeMap<String, Agent> {
        &self.agents
    }

    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    pub fn process(&self) -> &Process {
        &self.process
    }

    pub fn mermaid(&self) -> String {
        let mut lines = vec![
            "flowchart TD".to_string(),
            "  kickoff([Kickoff])".to_string(),
        ];

        for task in &self.tasks {
            lines.push(format!(
                "  {}[\"{}\\n{}\"]",
                sanitize_node_id(task.id()),
                task.id(),
                task.agent()
            ));
        }

        for task in &self.tasks {
            if task.depends_on().is_empty() {
                lines.push(format!("  kickoff --> {}", sanitize_node_id(task.id())));
            } else {
                for dependency in task.depends_on() {
                    lines.push(format!(
                        "  {} --> {}",
                        sanitize_node_id(dependency),
                        sanitize_node_id(task.id())
                    ));
                }
            }
        }

        lines.join("\n")
    }

    pub async fn kickoff<I>(&self, input: I) -> Result<CrewExecution>
    where
        I: Into<KickoffInput>,
    {
        let kickoff = input.into();
        let ordered_tasks = topological_tasks(&self.tasks)?;
        let mut outcomes: Vec<TaskOutcome> = Vec::with_capacity(ordered_tasks.len());
        let mut transcript = Vec::new();
        let mut usage = TokenUsage::default();

        for task in ordered_tasks {
            let agent = self
                .agents
                .get(task.agent())
                .cloned()
                .ok_or_else(|| CrewError::MissingAgent(task.agent().to_string()))?;

            let dependencies = task
                .depends_on()
                .iter()
                .map(|dependency| {
                    outcomes
                        .iter()
                        .find(|outcome| &outcome.task_id == dependency)
                        .cloned()
                        .ok_or_else(|| CrewError::MissingTask(dependency.clone()))
                })
                .collect::<Result<Vec<_>>>()?;

            transcript.push(TranscriptEvent {
                kind: TranscriptEventKind::TaskStarted,
                task_id: Some(task.id().to_string()),
                agent_id: Some(agent.id().to_string()),
                message: format!("{} started {}", agent.role(), task.id()),
            });

            let (manager_brief, manager_usage) = self
                .build_manager_brief(task, &dependencies, &kickoff)
                .await?;
            usage += manager_usage;

            if let Some(brief) = &manager_brief {
                transcript.push(TranscriptEvent {
                    kind: TranscriptEventKind::ManagerBriefed,
                    task_id: Some(task.id().to_string()),
                    agent_id: None,
                    message: brief.clone(),
                });
            }

            let outcome = execute_task(
                &agent,
                task,
                &kickoff,
                &dependencies,
                manager_brief,
                &mut transcript,
            )
            .await?;
            usage += outcome.usage.clone();

            transcript.push(TranscriptEvent {
                kind: TranscriptEventKind::TaskCompleted,
                task_id: Some(outcome.task_id.clone()),
                agent_id: Some(outcome.agent_id.clone()),
                message: format!("{} completed {}", outcome.agent_id, outcome.task_id),
            });

            outcomes.push(outcome);
        }

        let final_output = outcomes
            .last()
            .map(|outcome| outcome.output.clone())
            .ok_or_else(|| {
                CrewError::InvalidConfig("a crew must contain at least one task".to_string())
            })?;

        Ok(CrewExecution {
            crew_name: self.name.clone(),
            kickoff,
            final_output,
            tasks: outcomes,
            transcript,
            usage,
        })
    }

    async fn build_manager_brief(
        &self,
        task: &Task,
        dependencies: &[TaskOutcome],
        kickoff: &KickoffInput,
    ) -> Result<(Option<String>, TokenUsage)> {
        let Process::Hierarchical { manager } = &self.process else {
            return Ok((None, TokenUsage::default()));
        };

        let messages = vec![
            ModelMessage::system(format!(
                "You are the crew manager.\nRole: {}\nGoal: {}\nBackstory: {}\nReturn a concise operating brief for the next task.",
                manager.role(),
                manager.goal(),
                manager.backstory().unwrap_or("n/a")
            )),
            ModelMessage::user(render_manager_prompt(task, dependencies, kickoff)),
        ];

        let response = manager
            .model()
            .complete(ModelRequest {
                messages,
                temperature: manager.temperature(),
                max_tokens: Some(300),
                metadata: BTreeMap::new(),
            })
            .await?;

        let brief = unwrap_final_answer(&response.content);
        Ok((Some(brief), response.usage.unwrap_or_default()))
    }
}

#[derive(Debug, Clone)]
pub struct CrewBuilder {
    name: String,
    agents: BTreeMap<String, Agent>,
    tasks: Vec<Task>,
    process: Process,
}

impl CrewBuilder {
    pub fn agent(mut self, agent: Agent) -> Self {
        self.agents.insert(agent.id().to_string(), agent);
        self
    }

    pub fn task(mut self, task: Task) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn process(mut self, process: Process) -> Self {
        self.process = process;
        self
    }

    pub fn build(self) -> Result<Crew> {
        if self.name.trim().is_empty() {
            return Err(CrewError::InvalidConfig(
                "crew name cannot be empty".to_string(),
            ));
        }

        if self.tasks.is_empty() {
            return Err(CrewError::InvalidConfig(
                "a crew must contain at least one task".to_string(),
            ));
        }

        for task in &self.tasks {
            if !self.agents.contains_key(task.agent()) {
                return Err(CrewError::MissingAgent(task.agent().to_string()));
            }
        }

        topological_tasks(&self.tasks)?;

        Ok(Crew {
            name: self.name,
            agents: self.agents,
            tasks: self.tasks,
            process: self.process,
        })
    }
}

async fn execute_task(
    agent: &Agent,
    task: &Task,
    kickoff: &KickoffInput,
    dependencies: &[TaskOutcome],
    manager_brief: Option<String>,
    transcript: &mut Vec<TranscriptEvent>,
) -> Result<TaskOutcome> {
    let mut messages = vec![
        ModelMessage::system(render_agent_system_prompt(agent)),
        ModelMessage::user(render_task_prompt(
            task,
            kickoff,
            dependencies,
            manager_brief.as_deref(),
        )),
    ];
    let mut tool_calls = Vec::new();
    let mut usage = TokenUsage::default();

    for _ in 0..agent.max_iterations() {
        let response = agent
            .model()
            .complete(ModelRequest {
                messages: messages.clone(),
                temperature: agent.temperature(),
                max_tokens: None,
                metadata: BTreeMap::new(),
            })
            .await?;

        usage += response.usage.unwrap_or_default();
        let assistant_text = response.content.clone();
        messages.push(ModelMessage::assistant(assistant_text.clone()));

        if let Some(tool_call) = parse_tool_call(&assistant_text)? {
            let tool = agent
                .find_tool(&tool_call.tool)
                .ok_or_else(|| CrewError::MissingTool(tool_call.tool.clone()))?;
            let output = tool
                .call(ToolInput::new(
                    tool_call.input.clone(),
                    task.id(),
                    agent.id(),
                ))
                .await?;

            transcript.push(TranscriptEvent {
                kind: TranscriptEventKind::ToolCalled,
                task_id: Some(task.id().to_string()),
                agent_id: Some(agent.id().to_string()),
                message: format!("{} called {}", agent.id(), tool_call.tool),
            });

            tool_calls.push(ToolCallRecord {
                tool: tool_call.tool.clone(),
                input: tool_call.input.clone(),
                output: output.content.clone(),
            });

            messages.push(ModelMessage {
                role: MessageRole::Tool,
                content: format!(
                    "Tool `{}` returned the following result:\n{}",
                    tool_call.tool, output.content
                ),
            });
            continue;
        }

        let output = unwrap_final_answer(&assistant_text);
        return Ok(TaskOutcome {
            task_id: task.id().to_string(),
            agent_id: agent.id().to_string(),
            output,
            tool_calls,
            manager_brief,
            usage,
        });
    }

    Err(CrewError::MaxIterationsExceeded {
        agent: agent.id().to_string(),
        task: task.id().to_string(),
    })
}

fn render_agent_system_prompt(agent: &Agent) -> String {
    let tools = if agent.tools().is_empty() {
        "No tools are registered.".to_string()
    } else {
        agent
            .tools()
            .iter()
            .map(|tool| format!("- {}: {}", tool.name(), tool.description()))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "You are a crew member inside a Rust-native CrewAI-style runtime.\n\
Role: {}\n\
Goal: {}\n\
Backstory: {}\n\
Delegation allowed: {}\n\n\
Available tools:\n{}\n\n\
When you need a tool, respond with exactly one XML tag:\n\
<tool_call>{{\"tool\":\"tool_name\",\"input\":\"what to pass to the tool\"}}</tool_call>\n\
When you are done, either reply normally or wrap the final answer with:\n\
<final_answer>your final answer</final_answer>\n\
Be concrete, structured, and concise.",
        agent.role(),
        agent.goal(),
        agent.backstory().unwrap_or("n/a"),
        agent.allow_delegation(),
        tools
    )
}

fn render_task_prompt(
    task: &Task,
    kickoff: &KickoffInput,
    dependencies: &[TaskOutcome],
    manager_brief: Option<&str>,
) -> String {
    let kickoff_context = if kickoff.context.is_empty() {
        "none".to_string()
    } else {
        kickoff
            .context
            .iter()
            .map(|(key, value)| format!("- {}: {}", key, value))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let dependency_context = if dependencies.is_empty() {
        "none".to_string()
    } else {
        dependencies
            .iter()
            .map(|outcome| format!("### {}\n{}", outcome.task_id, outcome.output))
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    format!(
        "Crew goal:\n{}\n\n\
Task id: {}\n\
Task description:\n{}\n\n\
Expected output:\n{}\n\n\
Preferred output format: {:?}\n\n\
Task context:\n{}\n\n\
Manager brief:\n{}\n\n\
Kickoff context:\n{}\n\n\
Completed dependency outputs:\n{}",
        kickoff.goal,
        task.id(),
        task.description(),
        task.expected_output().unwrap_or("not specified"),
        task.output_format(),
        task.context().unwrap_or("none"),
        manager_brief.unwrap_or("none"),
        kickoff_context,
        dependency_context
    )
}

fn render_manager_prompt(
    task: &Task,
    dependencies: &[TaskOutcome],
    kickoff: &KickoffInput,
) -> String {
    let dependency_summary = if dependencies.is_empty() {
        "none".to_string()
    } else {
        dependencies
            .iter()
            .map(|outcome| {
                format!(
                    "- {} by {}: {}",
                    outcome.task_id, outcome.agent_id, outcome.output
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Crew goal:\n{}\n\n\
Upcoming task: {}\n\
Description: {}\n\
Expected output: {}\n\n\
Completed work:\n{}\n\n\
Respond with a short manager brief that sharpens scope, reduces drift, and mentions any risk to avoid.",
        kickoff.goal,
        task.id(),
        task.description(),
        task.expected_output().unwrap_or("not specified"),
        dependency_summary
    )
}

#[derive(Debug, Clone, Deserialize)]
struct ToolDirective {
    tool: String,
    input: String,
}

fn parse_tool_call(content: &str) -> Result<Option<ToolDirective>> {
    let Some(raw) = extract_tag(content, "tool_call") else {
        return Ok(None);
    };

    let directive = serde_json::from_str::<ToolDirective>(&raw)?;
    Ok(Some(directive))
}

fn unwrap_final_answer(content: &str) -> String {
    extract_tag(content, "final_answer").unwrap_or_else(|| content.trim().to_string())
}

fn extract_tag(content: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = content.find(&open)?;
    let rest = &content[start + open.len()..];
    let end = rest.find(&close)?;
    Some(rest[..end].trim().to_string())
}

fn topological_tasks(tasks: &[Task]) -> Result<Vec<&Task>> {
    let mut task_map = BTreeMap::new();
    let mut indegree = BTreeMap::new();
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();

    for task in tasks {
        task_map.insert(task.id(), task);
        indegree.entry(task.id()).or_insert(0usize);
    }

    for task in tasks {
        for dependency in task.depends_on() {
            if !task_map.contains_key(dependency.as_str()) {
                return Err(CrewError::MissingDependency {
                    task: task.id().to_string(),
                    dependency: dependency.clone(),
                });
            }

            *indegree
                .get_mut(task.id())
                .expect("task indegree should exist") += 1;
            edges
                .entry(dependency.as_str())
                .or_default()
                .push(task.id());
        }
    }

    let mut queue = VecDeque::new();
    for task in tasks {
        if indegree.get(task.id()).copied().unwrap_or_default() == 0 {
            queue.push_back(task.id());
        }
    }

    let mut ordered = Vec::with_capacity(tasks.len());
    while let Some(task_id) = queue.pop_front() {
        let task = task_map
            .get(task_id)
            .copied()
            .ok_or_else(|| CrewError::MissingTask(task_id.to_string()))?;
        ordered.push(task);

        if let Some(children) = edges.get(task_id) {
            for child in children {
                let child_degree = indegree
                    .get_mut(child)
                    .expect("child indegree should exist");
                *child_degree -= 1;
                if *child_degree == 0 {
                    queue.push_back(child);
                }
            }
        }
    }

    if ordered.len() != tasks.len() {
        return Err(CrewError::CyclicTaskGraph);
    }

    Ok(ordered)
}

fn sanitize_node_id(id: &str) -> String {
    id.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
