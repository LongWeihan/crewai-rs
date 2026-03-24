use async_trait::async_trait;
use crewai_rs::{Flow, FlowContext, FlowStep, FlowTransition};

#[derive(Debug, Default)]
struct State {
    validated: bool,
    published: bool,
}

struct Validate;
struct Publish;

#[async_trait]
impl FlowStep<State> for Validate {
    async fn run(
        &self,
        state: &mut State,
        _context: &FlowContext,
    ) -> crewai_rs::Result<FlowTransition> {
        state.validated = true;
        Ok(FlowTransition::Next("publish".to_string()))
    }
}

#[async_trait]
impl FlowStep<State> for Publish {
    async fn run(
        &self,
        state: &mut State,
        _context: &FlowContext,
    ) -> crewai_rs::Result<FlowTransition> {
        state.published = true;
        Ok(FlowTransition::Finish)
    }
}

#[tokio::test]
async fn typed_flows_can_drive_stateful_work() {
    let flow = Flow::builder("launch-flow")
        .step("validate", Validate)
        .step("publish", Publish)
        .edge("validate", "publish")
        .start("validate")
        .build()
        .unwrap();

    let result = flow.run(State::default()).await.unwrap();

    assert_eq!(result.history, vec!["validate", "publish"]);
    assert!(result.state.validated);
    assert!(result.state.published);
}
