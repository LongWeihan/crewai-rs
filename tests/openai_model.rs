use crewai_rs::{ChatModel, MessageRole, ModelMessage, ModelRequest, OpenAIChatModel};
use serde_json::json;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

#[tokio::test]
async fn openai_adapter_posts_chat_completion_requests() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [
                {
                    "message": {
                        "content": "<final_answer>Hello from the mock API.</final_answer>"
                    }
                }
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        })))
        .mount(&server)
        .await;

    let model = OpenAIChatModel::builder("gpt-test", "secret")
        .base_url(server.uri())
        .build()
        .unwrap();

    let response = model
        .complete(ModelRequest {
            messages: vec![
                ModelMessage::system("You are helpful."),
                ModelMessage {
                    role: MessageRole::User,
                    content: "Say hello.".to_string(),
                },
            ],
            temperature: Some(0.1),
            max_tokens: Some(32),
            metadata: Default::default(),
        })
        .await
        .unwrap();

    assert_eq!(
        response.content,
        "<final_answer>Hello from the mock API.</final_answer>"
    );
    assert_eq!(response.usage.unwrap().total_tokens, 15);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);

    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["model"], "gpt-test");
    assert_eq!(body["messages"][0]["role"], "system");
    assert_eq!(body["messages"][0]["content"], "You are helpful.");
    assert_eq!(body["messages"][1]["role"], "user");
    assert_eq!(body["messages"][1]["content"], "Say hello.");
    assert_eq!(body["max_tokens"], 32);
}
