use crate::helpers::{spawn_app, TestApp};
use reqwest::header::CONTENT_TYPE;
use reqwest::StatusCode;

// TODO: testing the db diff
//       A) belongs in a different test
//       B) belongs in e2e and unit tests, not integration tests
#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let TestApp {
        address,
        connection_pool,
    } = spawn_app().await;
    let mut connection = connection_pool
        .acquire()
        .await
        .expect("Failed to acquire connection");
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
    .fetch_optional(connection.as_mut())
    .await
    .expect("Failed to execute query");

    assert!(saved.is_some());

    let saved = saved.unwrap();

    assert_eq!("ursula_le_guin@ztpir.com", saved.email);
    assert_eq!("le guin", saved.name);
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let TestApp { address, .. } = spawn_app().await;
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

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40ztpir.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            StatusCode::BAD_REQUEST,
            response.status(),
            "The API did not return a 400 BAD_REQUEST when the payload was {description}"
        );
    }
}
