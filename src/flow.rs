use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CrewError, Result};

#[derive(Debug, Clone)]
pub struct FlowContext {
    pub execution_id: String,
    pub step_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "target", rename_all = "snake_case")]
pub enum FlowTransition {
    Next(String),
    Finish,
}

#[async_trait]
pub trait FlowStep<State>: Send + Sync {
    async fn run(&self, state: &mut State, context: &FlowContext) -> Result<FlowTransition>;
}

#[derive(Debug, Clone)]
pub struct FlowRun<State> {
    pub execution_id: String,
    pub history: Vec<String>,
    pub state: State,
}

pub struct Flow<State> {
    name: String,
    start: String,
    steps: BTreeMap<String, Arc<dyn FlowStep<State>>>,
    edges: Vec<(String, String)>,
    max_steps: usize,
}

impl<State> std::fmt::Debug for Flow<State> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flow")
            .field("name", &self.name)
            .field("start", &self.start)
            .field("steps", &self.steps.keys().collect::<Vec<_>>())
            .field("edges", &self.edges)
            .field("max_steps", &self.max_steps)
            .finish()
    }
}

impl<State> Flow<State> {
    pub fn builder(name: impl Into<String>) -> FlowBuilder<State> {
        FlowBuilder {
            name: name.into(),
            start: None,
            steps: BTreeMap::new(),
            edges: Vec::new(),
            max_steps: 32,
        }
    }

    pub async fn run(&self, mut state: State) -> Result<FlowRun<State>> {
        let execution_id = Uuid::new_v4().to_string();
        let mut current = self.start.clone();
        let mut history = Vec::new();

        for step_index in 0..self.max_steps {
            let step = self.steps.get(&current).ok_or_else(|| {
                CrewError::InvalidConfig(format!("flow step `{}` was not registered", current))
            })?;

            history.push(current.clone());
            let transition = step
                .run(
                    &mut state,
                    &FlowContext {
                        execution_id: execution_id.clone(),
                        step_index,
                    },
                )
                .await?;

            match transition {
                FlowTransition::Finish => {
                    return Ok(FlowRun {
                        execution_id,
                        history,
                        state,
                    });
                }
                FlowTransition::Next(next) => current = next,
            }
        }

        Err(CrewError::InvalidConfig(format!(
            "flow `{}` exceeded max step count of {}",
            self.name, self.max_steps
        )))
    }

    pub fn mermaid(&self) -> String {
        let mut lines = vec!["flowchart LR".to_string()];
        for step in self.steps.keys() {
            lines.push(format!("  {}([{}])", sanitize_step_id(step), step));
        }

        for (from, to) in &self.edges {
            lines.push(format!(
                "  {} --> {}",
                sanitize_step_id(from),
                sanitize_step_id(to)
            ));
        }

        lines.join("\n")
    }
}

pub struct FlowBuilder<State> {
    name: String,
    start: Option<String>,
    steps: BTreeMap<String, Arc<dyn FlowStep<State>>>,
    edges: Vec<(String, String)>,
    max_steps: usize,
}

impl<State> FlowBuilder<State> {
    pub fn start(mut self, step: impl Into<String>) -> Self {
        self.start = Some(step.into());
        self
    }

    pub fn step<S>(mut self, name: impl Into<String>, step: S) -> Self
    where
        S: FlowStep<State> + 'static,
    {
        self.steps.insert(name.into(), Arc::new(step));
        self
    }

    pub fn edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.edges.push((from.into(), to.into()));
        self
    }

    pub fn max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub fn build(self) -> Result<Flow<State>> {
        let start = self
            .start
            .ok_or_else(|| CrewError::InvalidConfig("flow start step is required".to_string()))?;

        if !self.steps.contains_key(&start) {
            return Err(CrewError::InvalidConfig(format!(
                "flow start step `{}` is not registered",
                start
            )));
        }

        for (from, to) in &self.edges {
            if !self.steps.contains_key(from) {
                return Err(CrewError::InvalidConfig(format!(
                    "flow edge source `{}` is not registered",
                    from
                )));
            }

            if !self.steps.contains_key(to) {
                return Err(CrewError::InvalidConfig(format!(
                    "flow edge target `{}` is not registered",
                    to
                )));
            }
        }

        Ok(Flow {
            name: self.name,
            start,
            steps: self.steps,
            edges: self.edges,
            max_steps: self.max_steps,
        })
    }
}

fn sanitize_step_id(step: &str) -> String {
    step.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
