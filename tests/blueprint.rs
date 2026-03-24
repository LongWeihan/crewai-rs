use std::sync::Arc;

use crewai_rs::{CrewBlueprint, FnTool, MockChatModel, RuntimeRegistry, ToolOutput};

#[tokio::test]
async fn yaml_blueprints_build_crews_from_runtime_registries() {
    let blueprint = CrewBlueprint::from_yaml_str(
        r#"
name: launch-crew
process: hierarchical
manager:
  id: manager
  role: Product Strategist
  goal: Keep the crew focused.
  model: planner
agents:
  - id: researcher
    role: Rust OSS Researcher
    goal: Find launch hooks.
    model: planner
    tools: [search]
  - id: writer
    role: Launch Writer
    goal: Write the final brief.
    model: writer
tasks:
  - id: research
    description: Research the launch angle.
    agent: researcher
  - id: brief
    description: Write the final brief.
    agent: writer
    depends_on: [research]
"#,
    )
    .unwrap();

    let planner = Arc::new(MockChatModel::from_strings(
        "planner",
        [
            "<final_answer>Push the crew toward a strong commercial message.</final_answer>",
            r#"<tool_call>{"tool":"search","input":"CrewAI-rs launch hooks"}</tool_call>"#,
            "<final_answer>- Rust is underserved\n- Typed flows matter</final_answer>",
            "<final_answer>Keep the final brief commercially sharp and concise.</final_answer>",
        ],
    ));
    let writer = Arc::new(MockChatModel::from_strings(
        "writer",
        [
            "<final_answer>Ship crewai-rs as the typed crew runtime for Rust AI products.</final_answer>",
        ],
    ));

    let registry = RuntimeRegistry::new()
        .with_model("planner", planner)
        .with_model("writer", writer)
        .with_tool(
            "search",
            Arc::new(FnTool::new(
                "search",
                "Looks up demand.",
                |input| async move { Ok(ToolOutput::text(format!("found {}", input.value))) },
            )),
        );

    let crew = blueprint.build(&registry).unwrap();
    let result = crew.kickoff("Launch crewai-rs.").await.unwrap();

    assert!(result.final_output.contains("typed crew runtime"));
    assert_eq!(result.tasks[0].tool_calls.len(), 1);
}
