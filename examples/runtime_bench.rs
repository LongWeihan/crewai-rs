use std::{
    hint::black_box,
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use crewai_rs::{
    Agent, ChatModel, Crew, CrewBlueprint, Flow, FlowContext, FlowStep, FlowTransition,
    KickoffInput, MessageRole, ModelRequest, ModelResponse, Process, RuntimeRegistry, Task,
    ToolInput, ToolOutput,
};

const BLUEPRINT_YAML: &str = r#"
name: launch-crew
process: hierarchical
manager:
  id: manager
  role: Product Strategist
  goal: Keep the crew focused.
  model: manager
agents:
  - id: researcher
    role: Rust OSS Researcher
    goal: Find launch hooks.
    model: researcher
    tools: [search]
  - id: writer
    role: Launch Writer
    goal: Write the final brief.
    model: writer
tasks:
  - id: research
    description: Research the launch angle.
    expected_output: Bullet findings.
    agent: researcher
  - id: brief
    description: Write the final brief.
    expected_output: Markdown brief.
    agent: writer
    depends_on: [research]
"#;

#[derive(Clone)]
struct ResearchModel;

#[async_trait]
impl ChatModel for ResearchModel {
    fn name(&self) -> &str {
        "research-model"
    }

    async fn complete(&self, request: ModelRequest) -> crewai_rs::Result<ModelResponse> {
        let saw_tool_result = request
            .messages
            .iter()
            .any(|message| matches!(message.role, MessageRole::Tool));

        if saw_tool_result {
            Ok(ModelResponse::text(
                "<final_answer>- Rust demand exists\n- Typed orchestration is differentiating</final_answer>",
            ))
        } else {
            Ok(ModelResponse::text(
                r#"<tool_call>{"tool":"search","input":"CrewAI-rs launch angle"}</tool_call>"#,
            ))
        }
    }
}

#[derive(Clone)]
struct WriterModel;

#[async_trait]
impl ChatModel for WriterModel {
    fn name(&self) -> &str {
        "writer-model"
    }

    async fn complete(&self, _request: ModelRequest) -> crewai_rs::Result<ModelResponse> {
        Ok(ModelResponse::text(
            "<final_answer>Ship crewai-rs as the typed orchestration layer for Rust AI products.</final_answer>",
        ))
    }
}

#[derive(Clone)]
struct ManagerModel;

#[async_trait]
impl ChatModel for ManagerModel {
    fn name(&self) -> &str {
        "manager-model"
    }

    async fn complete(&self, _request: ModelRequest) -> crewai_rs::Result<ModelResponse> {
        Ok(ModelResponse::text(
            "<final_answer>Keep the final brief commercially sharp and concise.</final_answer>",
        ))
    }
}

#[derive(Clone)]
struct SearchTool;

#[async_trait]
impl crewai_rs::Tool for SearchTool {
    fn name(&self) -> &str {
        "search"
    }

    fn description(&self) -> &str {
        "Returns a deterministic search result for benchmarks."
    }

    async fn call(&self, input: ToolInput) -> crewai_rs::Result<ToolOutput> {
        Ok(ToolOutput::text(format!(
            "search result for `{}`",
            input.value
        )))
    }
}

#[derive(Default)]
struct FlowState {
    score: u32,
    published: bool,
}

struct ValidateIdea;
struct Publish;

#[async_trait]
impl FlowStep<FlowState> for ValidateIdea {
    async fn run(
        &self,
        state: &mut FlowState,
        _context: &FlowContext,
    ) -> crewai_rs::Result<FlowTransition> {
        state.score = 9;
        Ok(FlowTransition::Next("publish".to_string()))
    }
}

#[async_trait]
impl FlowStep<FlowState> for Publish {
    async fn run(
        &self,
        state: &mut FlowState,
        _context: &FlowContext,
    ) -> crewai_rs::Result<FlowTransition> {
        state.published = true;
        Ok(FlowTransition::Finish)
    }
}

#[derive(Debug, Clone)]
struct Stats {
    label: &'static str,
    iterations: usize,
    mean: Duration,
    median: Duration,
    p95: Duration,
    ops_per_sec: f64,
}

impl Stats {
    fn to_markdown_row(&self, baseline: Option<Duration>) -> String {
        let relative = baseline
            .map(|base| format!("{:.2}x", self.mean.as_secs_f64() / base.as_secs_f64()))
            .unwrap_or_else(|| "1.00x".to_string());

        format!(
            "| `{}` | {} | {} | {} | {} | {:.0} | {} |",
            self.label,
            self.iterations,
            format_duration(self.mean),
            format_duration(self.median),
            format_duration(self.p95),
            self.ops_per_sec,
            relative
        )
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> crewai_rs::Result<()> {
    let flow = build_flow()?;
    let sequential = build_sequential_crew()?;
    let hierarchical = build_hierarchical_crew()?;
    let registry = build_registry();
    let blueprint = CrewBlueprint::from_yaml_str(BLUEPRINT_YAML)?;

    let flow_stats = measure("flow_run", 20_000, 500, || async {
        let run = flow.run(FlowState::default()).await?;
        black_box(run.history.len());
        Ok(())
    })
    .await?;

    let sequential_stats = measure("crew_kickoff_sequential", 5_000, 200, || async {
        let execution = sequential
            .kickoff(KickoffInput::new("Launch crewai-rs."))
            .await?;
        black_box(execution.final_output.len());
        Ok(())
    })
    .await?;

    let hierarchical_stats = measure("crew_kickoff_hierarchical", 5_000, 200, || async {
        let execution = hierarchical
            .kickoff(KickoffInput::new("Launch crewai-rs."))
            .await?;
        black_box(execution.final_output.len());
        Ok(())
    })
    .await?;

    let blueprint_stats = measure("blueprint_parse_and_build", 8_000, 200, || async {
        let parsed = CrewBlueprint::from_yaml_str(BLUEPRINT_YAML)?;
        let built = parsed.build(&registry)?;
        black_box(built.tasks().len());
        Ok(())
    })
    .await?;

    let stats = [
        flow_stats,
        sequential_stats,
        hierarchical_stats,
        blueprint_stats,
    ];

    println!("Benchmark methodology:");
    println!("- release mode");
    println!("- single-process local microbenchmark");
    println!("- no network I/O; deterministic in-memory models and tools only");
    println!("- each scenario includes warmup iterations before measurement");
    println!();
    println!("| Scenario | Iterations | Mean | Median | p95 | Ops/s | Relative to flow |");
    println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: |");

    let baseline = stats[0].mean;
    for stat in &stats {
        println!("{}", stat.to_markdown_row(Some(baseline)));
    }

    let hierarchical_delta =
        (stats[2].mean.as_secs_f64() / stats[1].mean.as_secs_f64() - 1.0) * 100.0;
    println!();
    println!(
        "Manager overhead versus sequential kickoff: +{:.1}%",
        hierarchical_delta
    );

    black_box(blueprint);
    Ok(())
}

async fn measure<F, Fut>(
    label: &'static str,
    iterations: usize,
    warmup: usize,
    mut op: F,
) -> crewai_rs::Result<Stats>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = crewai_rs::Result<()>>,
{
    for _ in 0..warmup {
        op().await?;
    }

    let mut samples = Vec::with_capacity(iterations);
    let started = Instant::now();

    for _ in 0..iterations {
        let sample_started = Instant::now();
        op().await?;
        samples.push(sample_started.elapsed());
    }

    let elapsed = started.elapsed();
    samples.sort_unstable();

    let mean = Duration::from_secs_f64(elapsed.as_secs_f64() / iterations as f64);
    let median = samples[iterations / 2];
    let p95 = samples[((iterations as f64) * 0.95) as usize];
    let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

    Ok(Stats {
        label,
        iterations,
        mean,
        median,
        p95,
        ops_per_sec,
    })
}

fn format_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    if nanos >= 1_000_000 {
        format!("{:.3} ms", duration.as_secs_f64() * 1_000.0)
    } else if nanos >= 1_000 {
        format!("{:.1} us", duration.as_secs_f64() * 1_000_000.0)
    } else {
        format!("{} ns", nanos)
    }
}

fn build_registry() -> RuntimeRegistry {
    RuntimeRegistry::new()
        .with_model("researcher", Arc::new(ResearchModel))
        .with_model("writer", Arc::new(WriterModel))
        .with_model("manager", Arc::new(ManagerModel))
        .with_tool("search", Arc::new(SearchTool))
}

fn build_flow() -> crewai_rs::Result<Flow<FlowState>> {
    Flow::builder("launch-flow")
        .step("validate", ValidateIdea)
        .step("publish", Publish)
        .edge("validate", "publish")
        .start("validate")
        .build()
}

fn build_sequential_crew() -> crewai_rs::Result<Crew> {
    let search_tool = Arc::new(SearchTool);

    let researcher = Agent::builder("researcher")
        .role("Researcher")
        .goal("Find launch hooks.")
        .model_ref(Arc::new(ResearchModel))
        .tool_ref(search_tool)
        .build()?;

    let writer = Agent::builder("writer")
        .role("Writer")
        .goal("Write the final brief.")
        .model_ref(Arc::new(WriterModel))
        .build()?;

    Crew::builder("launch-crew")
        .agent(researcher)
        .agent(writer)
        .task(
            Task::builder("research")
                .description("Research the launch angle.")
                .expected_output("Bullet findings.")
                .agent("researcher")
                .build()?,
        )
        .task(
            Task::builder("brief")
                .description("Write the final brief.")
                .expected_output("Markdown brief.")
                .agent("writer")
                .depends_on(["research"])
                .build()?,
        )
        .build()
}

fn build_hierarchical_crew() -> crewai_rs::Result<Crew> {
    let search_tool = Arc::new(SearchTool);

    let researcher = Agent::builder("researcher")
        .role("Researcher")
        .goal("Find launch hooks.")
        .model_ref(Arc::new(ResearchModel))
        .tool_ref(search_tool)
        .build()?;

    let writer = Agent::builder("writer")
        .role("Writer")
        .goal("Write the final brief.")
        .model_ref(Arc::new(WriterModel))
        .build()?;

    let manager = Agent::builder("manager")
        .role("Manager")
        .goal("Keep the crew focused.")
        .model_ref(Arc::new(ManagerModel))
        .build()?;

    Crew::builder("launch-crew")
        .agent(researcher)
        .agent(writer)
        .task(
            Task::builder("research")
                .description("Research the launch angle.")
                .expected_output("Bullet findings.")
                .agent("researcher")
                .build()?,
        )
        .task(
            Task::builder("brief")
                .description("Write the final brief.")
                .expected_output("Markdown brief.")
                .agent("writer")
                .depends_on(["research"])
                .build()?,
        )
        .process(Process::Hierarchical { manager })
        .build()
}
