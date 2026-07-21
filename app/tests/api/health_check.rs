use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn health_check_returns_ok() {
    let TestApp { address, .. } = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{address}/healthz"))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
}
