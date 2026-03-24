use async_trait::async_trait;
use crewai_rs::{Flow, FlowContext, FlowStep, FlowTransition};

#[derive(Debug, Default)]
struct LaunchState {
    score: u32,
    ready: bool,
}

struct ValidateIdea;
struct Publish;

#[async_trait]
impl FlowStep<LaunchState> for ValidateIdea {
    async fn run(
        &self,
        state: &mut LaunchState,
        _context: &FlowContext,
    ) -> crewai_rs::Result<FlowTransition> {
        state.score = 9;
        state.ready = true;
        Ok(FlowTransition::Next("publish".to_string()))
    }
}

#[async_trait]
impl FlowStep<LaunchState> for Publish {
    async fn run(
        &self,
        state: &mut LaunchState,
        _context: &FlowContext,
    ) -> crewai_rs::Result<FlowTransition> {
        if state.ready {
            Ok(FlowTransition::Finish)
        } else {
            Ok(FlowTransition::Next("validate".to_string()))
        }
    }
}

#[tokio::main]
async fn main() -> crewai_rs::Result<()> {
    let flow = Flow::builder("launch-flow")
        .step("validate", ValidateIdea)
        .step("publish", Publish)
        .edge("validate", "publish")
        .start("validate")
        .build()?;

    let result = flow.run(LaunchState::default()).await?;
    println!("history: {:?}", result.history);
    println!("score: {}", result.state.score);
    Ok(())
}
