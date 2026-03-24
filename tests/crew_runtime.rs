use std::sync::Arc;

use crewai_rs::{Agent, Crew, FnTool, KickoffInput, MockChatModel, Process, Task, ToolOutput};

#[tokio::test]
async fn sequential_crews_execute_tools_and_dependency_context() {
    let search_tool = Arc::new(FnTool::new(
        "search",
        "Searches launch data.",
        |input| async move {
            Ok(ToolOutput::text(format!(
                "launch insight for `{}`",
                input.value
            )))
        },
    ));

    let researcher_model = Arc::new(MockChatModel::from_strings(
        "researcher-model",
        [
            r#"<tool_call>{"tool":"search","input":"Rust crew launch"}</tool_call>"#,
            r#"<final_answer>- Rust demand exists
- People want typed orchestration</final_answer>"#,
        ],
    ));

    let writer_model = Arc::new(MockChatModel::from_strings(
        "writer-model",
        [
            r#"<final_answer>Launch crewai-rs as the typed orchestration layer for Rust AI teams.</final_answer>"#,
        ],
    ));

    let crew = Crew::builder("launch")
        .agent(
            Agent::builder("researcher")
                .role("Researcher")
                .goal("Find demand signals.")
                .model_ref(researcher_model.clone())
                .tool_ref(search_tool)
                .build()
                .unwrap(),
        )
        .agent(
            Agent::builder("writer")
                .role("Writer")
                .goal("Turn research into messaging.")
                .model_ref(writer_model.clone())
                .build()
                .unwrap(),
        )
        .task(
            Task::builder("research")
                .description("Research demand.")
                .expected_output("Bullet list.")
                .agent("researcher")
                .build()
                .unwrap(),
        )
        .task(
            Task::builder("brief")
                .description("Write the launch brief.")
                .agent("writer")
                .depends_on(["research"])
                .build()
                .unwrap(),
        )
        .build()
        .unwrap();

    let execution = crew
        .kickoff(KickoffInput::new("Launch crewai-rs.").with_context("channel", "GitHub"))
        .await
        .unwrap();

    assert!(execution.final_output.contains("typed orchestration"));
    assert_eq!(execution.tasks[0].tool_calls.len(), 1);
    assert!(
        execution.tasks[0].tool_calls[0]
            .output
            .contains("launch insight")
    );

    let writer_requests = writer_model.requests();
    let last_user_message = writer_requests[0]
        .messages
        .iter()
        .find(|message| matches!(message.role, crewai_rs::MessageRole::User))
        .unwrap()
        .content
        .clone();
    assert!(last_user_message.contains("Rust demand exists"));
}

#[tokio::test]
async fn hierarchical_crews_request_manager_briefs() {
    let manager_model = Arc::new(MockChatModel::from_strings(
        "manager-model",
        [
            r#"<final_answer>Stay sharp: focus on the commercial angle and avoid generic AI claims.</final_answer>"#,
        ],
    ));

    let worker_model = Arc::new(MockChatModel::from_strings(
        "worker-model",
        [r#"<final_answer>Rust-native crews can be deployed as a single binary.</final_answer>"#],
    ));

    let crew = Crew::builder("hierarchical")
        .agent(
            Agent::builder("operator")
                .role("Operator")
                .goal("Execute the assigned task.")
                .model_ref(worker_model)
                .build()
                .unwrap(),
        )
        .task(
            Task::builder("summarize")
                .description("Summarize the launch angle.")
                .agent("operator")
                .build()
                .unwrap(),
        )
        .process(Process::Hierarchical {
            manager: Agent::builder("manager")
                .role("Manager")
                .goal("Keep the team commercially focused.")
                .model_ref(manager_model)
                .build()
                .unwrap(),
        })
        .build()
        .unwrap();

    let execution = crew.kickoff("Prepare launch copy.").await.unwrap();

    assert_eq!(
        execution.tasks[0].manager_brief.as_deref(),
        Some("Stay sharp: focus on the commercial angle and avoid generic AI claims.")
    );
    assert!(
        execution
            .transcript
            .iter()
            .any(|event| event.kind == crewai_rs::TranscriptEventKind::ManagerBriefed)
    );
}
