#![forbid(unsafe_code)]
//! Rust-native multi-agent orchestration primitives inspired by CrewAI.
//!
//! The crate is intentionally small but usable:
//! - `Agent`, `Task`, and `Crew` for orchestrating multi-step work
//! - `Tool` abstractions for model-directed tool use
//! - `Flow` for typed stateful workflows
//! - `CrewBlueprint` for YAML-driven project configuration
//! - `OpenAIChatModel` and `MockChatModel` model adapters

pub mod agent;
pub mod crew;
pub mod error;
pub mod flow;
pub mod model;
pub mod spec;
pub mod task;
pub mod tool;

pub use agent::{Agent, AgentBuilder};
pub use crew::{
    Crew, CrewBuilder, CrewExecution, KickoffInput, Process, TaskOutcome, TranscriptEvent,
    TranscriptEventKind,
};
pub use error::{CrewError, Result};
pub use flow::{Flow, FlowBuilder, FlowContext, FlowRun, FlowStep, FlowTransition};
pub use model::{
    ChatModel, MessageRole, MockChatModel, ModelMessage, ModelRequest, ModelResponse,
    OpenAIChatModel, OpenAIChatModelBuilder, TokenUsage,
};
pub use spec::{AgentBlueprint, CrewBlueprint, ProcessKind, RuntimeRegistry, TaskBlueprint};
pub use task::{OutputFormat, Task, TaskBuilder};
pub use tool::{FnTool, Tool, ToolCallRecord, ToolInput, ToolOutput};

pub mod prelude {
    pub use crate::{
        Agent, AgentBlueprint, ChatModel, Crew, CrewBlueprint, CrewExecution, Flow, FlowBuilder,
        FlowContext, FlowRun, FlowStep, FlowTransition, FnTool, KickoffInput, MessageRole,
        MockChatModel, ModelMessage, OpenAIChatModel, OutputFormat, Process, ProcessKind,
        RuntimeRegistry, Task, TaskBlueprint, ToolInput, ToolOutput,
    };
}
