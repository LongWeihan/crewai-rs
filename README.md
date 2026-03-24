# crewai-rs

<p align="center">
  <img src="./assets/hero.svg" alt="crewai-rs hero illustration" width="960" />
</p>

<p align="center">
  <a href="https://github.com/LongWeihan/crewai-rs/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/LongWeihan/crewai-rs/actions/workflows/ci.yml/badge.svg" /></a>
  <a href="https://github.com/LongWeihan/crewai-rs/stargazers"><img alt="GitHub stars" src="https://img.shields.io/github/stars/LongWeihan/crewai-rs?style=flat&logo=github" /></a>
  <img alt="status" src="https://img.shields.io/badge/status-alpha-c8553d" />
  <a href="./LICENSE"><img alt="license" src="https://img.shields.io/badge/license-MIT-0b132b" /></a>
  <img alt="rust" src="https://img.shields.io/badge/rust-1.85%2B-c8553d?logo=rust" />
  <img alt="OpenAI compatible" src="https://img.shields.io/badge/OpenAI-compatible-183A37" />
</p>

Rust-native multi-agent orchestration inspired by CrewAI.

`crewai-rs` gives you a practical MVP stack for building agent crews in Rust:

- typed `Agent`, `Task`, and `Crew` primitives
- sequential and hierarchical execution
- tool-calling with a lightweight XML/JSON protocol
- YAML blueprints for config-driven crews
- typed `Flow` state machines for deterministic workflow steps
- `MockChatModel` for deterministic tests
- `OpenAIChatModel` for real deployments and OpenAI-compatible endpoints

## Why this exists

The current AI orchestration ecosystem is still heavily Python-first. That is fine for experiments, but not ideal for:

- low-latency task orchestration
- stable binary deployments
- stronger type guarantees across flows and state
- production services that need concurrency without runtime surprises

`crewai-rs` is built to be idiomatic Rust first, not a line-by-line port.

## Install

```toml
[dependencies]
crewai-rs = "0.1.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Quick Start

```rust
use std::sync::Arc;

use crewai_rs::prelude::*;

#[tokio::main]
async fn main() -> crewai_rs::Result<()> {
    let search = Arc::new(FnTool::new(
        "search",
        "Find launch signals from the web index.",
        |input| async move {
            Ok(ToolOutput::text(format!(
                "Top hit for `{}`: Rust developers want a typed CrewAI alternative.",
                input.value
            )))
        },
    ));

    let researcher_model = Arc::new(MockChatModel::from_strings(
        "mock-researcher",
        [
            r#"<tool_call>{"tool":"search","input":"CrewAI Rust launch opportunities"}</tool_call>"#,
            r#"<final_answer>- Demand is real
- Typed flows are a differentiator
- Early adopters care about OpenAI-compatible backends</final_answer>"#,
        ],
    ));

    let writer_model = Arc::new(MockChatModel::from_strings(
        "mock-writer",
        [r#"<final_answer># Launch Brief

Ship `crewai-rs` as the Rust-native way to build crews, typed flows, and durable AI orchestration.</final_answer>"#],
    ));

    let researcher = Agent::builder("researcher")
        .role("Launch Researcher")
        .goal("Find the sharpest positioning for a Rust AI crew framework.")
        .backstory("You turn weak launch ideas into high-signal product narratives.")
        .model_ref(researcher_model)
        .tool_ref(search)
        .build()?;

    let writer = Agent::builder("writer")
        .role("Technical Writer")
        .goal("Turn research into a polished launch-ready brief.")
        .model_ref(writer_model)
        .build()?;

    let research = Task::builder("research-market")
        .description("Research the market gap for a Rust-native CrewAI-style framework.")
        .expected_output("A concise markdown bullet list with launch hooks.")
        .agent("researcher")
        .build()?;

    let brief = Task::builder("write-brief")
        .description("Write a crisp launch brief for GitHub and X.")
        .expected_output("A markdown brief with one clear positioning sentence.")
        .agent("writer")
        .depends_on(["research-market"])
        .build()?;

    let crew = Crew::builder("launch-crew")
        .agent(researcher)
        .agent(writer)
        .task(research)
        .task(brief)
        .build()?;

    let result = crew
        .kickoff(
            KickoffInput::new("Prepare the strongest possible public launch story.")
                .with_context("target_audience", "Rust builders shipping AI products"),
        )
        .await?;

    println!("{}", result.final_output);
    Ok(())
}
```

Run the example in this repo:

```bash
cargo run --example mock_launch
```

## YAML Blueprints

`crewai-rs` lets you define a crew in YAML, then attach live models and tools from a runtime registry.

```yaml
name: launch-crew
process: hierarchical
manager:
  id: manager
  role: Product Strategist
  goal: Keep the crew focused and commercially sharp.
  model: planner
agents:
  - id: researcher
    role: Rust OSS Researcher
    goal: Find demand signals and differentiators.
    model: planner
    tools: [search]
  - id: writer
    role: Launch Writer
    goal: Turn findings into launch copy.
    model: writer
tasks:
  - id: research-market
    description: Research launch opportunities for crewai-rs.
    expected_output: Bullet findings with proof points.
    agent: researcher
  - id: write-brief
    description: Turn findings into a launch brief.
    expected_output: Markdown brief.
    agent: writer
    depends_on: [research-market]
```

## Typed Flows

Agent crews are not the whole story. Sometimes you need deterministic workflow around them.

`Flow<State>` gives you typed orchestration for stateful steps such as:

- pre-flight validation
- routing between crews
- gating human approval
- publishing or persisting final outputs

See [`examples/flow_pipeline.rs`](./examples/flow_pipeline.rs) for a full example.

## Architecture

<p align="center">
  <img src="./assets/architecture.svg" alt="crewai-rs architecture diagram" width="960" />
</p>

## OpenAI-Compatible Models

Use the built-in adapter for real runs:

```rust
use crewai_rs::OpenAIChatModel;

let model = OpenAIChatModel::builder("gpt-4.1-mini", std::env::var("OPENAI_API_KEY")?)
    .temperature(0.2)
    .build()?;
```

The adapter talks to `/v1/chat/completions`, which also makes it usable with many OpenAI-compatible providers.

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features --all-targets
```

## Roadmap

- streamed model events and callbacks
- provider-native tool calling
- persistent memory backends
- structured JSON task outputs
- CLI scaffolding for blueprint projects

## License

MIT
