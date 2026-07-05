fn spawn_app() {
    let server = ztpir::run().expect("Failed to spawn server");
    tokio::spawn(server);
}

#[tokio::test]
async fn health_check_returns_ok() {
    spawn_app();
    let client = reqwest::Client::new();
    let response = client
        .get("http://127.0.0.1:8080/healthz")
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
}
