use thiserror::Error;

pub type Result<T> = std::result::Result<T, CrewError>;

#[derive(Debug, Error)]
pub enum CrewError {
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("agent `{0}` was not found")]
    MissingAgent(String),
    #[error("task `{0}` was not found")]
    MissingTask(String),
    #[error("tool `{0}` was not found")]
    MissingTool(String),
    #[error("model `{0}` was not found in the runtime registry")]
    MissingModel(String),
    #[error("task `{task}` depends on missing task `{dependency}`")]
    MissingDependency { task: String, dependency: String },
    #[error("task graph contains a cycle")]
    CyclicTaskGraph,
    #[error("agent `{agent}` exceeded max iterations while running task `{task}`")]
    MaxIterationsExceeded { agent: String, task: String },
    #[error("model error: {0}")]
    Model(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
