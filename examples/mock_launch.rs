use std::sync::Arc;

use crewai_rs::prelude::*;

#[tokio::main]
async fn main() -> crewai_rs::Result<()> {
    let search = Arc::new(FnTool::new(
        "search",
        "Searches launch demand signals.",
        |input| async move {
            Ok(ToolOutput::text(format!(
                "Search summary for `{}`: typed agent orchestration in Rust is still underserved.",
                input.value
            )))
        },
    ));

    let researcher = Agent::builder("researcher")
        .role("Rust Market Researcher")
        .goal("Find the strongest positioning for a Rust-native CrewAI alternative.")
        .backstory(
            "You hunt for high-signal demand indicators and compress them into useful bullets.",
        )
        .model_ref(Arc::new(MockChatModel::from_strings(
            "researcher-model",
            [
                r#"<tool_call>{"tool":"search","input":"CrewAI-rs launch angle"}</tool_call>"#,
                r#"<final_answer>- Developers want a stronger type system around crews
- Rust-native binaries lower deployment friction
- OpenAI-compatible endpoints make migration easier</final_answer>"#,
            ],
        )))
        .tool_ref(search)
        .build()?;

    let writer = Agent::builder("writer")
        .role("Launch Writer")
        .goal("Write a sharp launch brief from the research.")
        .model_ref(Arc::new(MockChatModel::from_strings(
            "writer-model",
            [r#"<final_answer># crewai-rs

The Rust-native way to ship AI crews, typed flows, and production-ready orchestration.</final_answer>"#],
        )))
        .build()?;

    let crew = Crew::builder("launch-crew")
        .agent(researcher)
        .agent(writer)
        .task(
            Task::builder("research-market")
                .description("Research the best public positioning for crewai-rs.")
                .expected_output("Three launch bullets with clear differentiation.")
                .agent("researcher")
                .build()?,
        )
        .task(
            Task::builder("write-brief")
                .description("Write a launch brief for GitHub and social distribution.")
                .expected_output("A headline and one short paragraph.")
                .agent("writer")
                .depends_on(["research-market"])
                .build()?,
        )
        .build()?;

    let result = crew
        .kickoff(
            KickoffInput::new("Prepare the public launch for crewai-rs.")
                .with_context("channel", "GitHub + X")
                .with_context("voice", "direct and technical"),
        )
        .await?;

    println!("{}", result.to_markdown());
    Ok(())
}
