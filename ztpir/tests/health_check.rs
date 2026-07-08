use reqwest::StatusCode;
use reqwest::header::CONTENT_TYPE;
use sqlx::{Connection, PgConnection};
use std::net::TcpListener;

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind listener");
    let port = listener
        .local_addr()
        .expect("listener bound without an address")
        .port();
    let server = ztpir::startup::run_server(listener).expect("Failed to spawn server");
    tokio::spawn(server);
    format!("http://127.0.0.1:{port}")
}

#[tokio::test]
async fn health_check_returns_ok() {
    let address = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{address}/healthz"))
        .send()
        .await
        .expect("Failed to send request");

    assert!(response.status().is_success());
}

// TODO: testing the db diff
//       A) belongs in a different test
//       B) belongs in e2e and unit tests, not integration tests
#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let address = spawn_app();
    let config = ztpir::configuration::get_config().expect("Failed to read configuration.");
    let connection_string = config.database.connection_string();
    let mut connection = PgConnection::connect(&connection_string)
        .await
        .expect("Failed to connect to database");
    sqlx::query!(
        "DELETE FROM subscriptions WHERE email = $1",
        "ursula_le_guin@ztpir.com"
    )
    .execute(&mut connection)
    .await
    .expect("Failed to execute query");
    let client = reqwest::Client::new();

    // TODO: urlencode with lib
    let body = "name=le%20guin&email=ursula_le_guin%40ztpir.com";
    let response = client
        .post(&format!("{address}/subscriptions"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(StatusCode::OK, response.status());

    let saved = sqlx::query!(
        "SELECT email, name FROM subscriptions WHERE email = $1",
        "ursula_le_guin@ztpir.com"
    )
    .fetch_optional(&mut connection)
    .await
    .expect("Failed to execute query");

    assert!(saved.is_some());

    let saved = saved.unwrap();

    assert_eq!("ursula_le_guin@ztpir.com", saved.email);
    assert_eq!("le guin", saved.name);
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let address = spawn_app();
    let client = reqwest::Client::new();
    let cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40ztpir.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, error_description) in cases {
        let response = client
            .post(&format!("{address}/subscriptions"))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            StatusCode::BAD_REQUEST,
            response.status(),
            "The API did not fail with 400 Bad Request when the payload was {error_description}"
        );
    }
}
