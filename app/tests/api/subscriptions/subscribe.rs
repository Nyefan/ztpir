use super::VALID_SUBSCRIPTION_PAYLOAD;
use crate::helpers::{ConfirmationLinks, spawn_app};
use app::domain::SubscriptionStatus;
use reqwest::StatusCode;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};
// TODO: urlencode with lib

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .mount(&app.email_server)
        .await;

    let response = app
        .post_subscriptions_subscribe(VALID_SUBSCRIPTION_PAYLOAD.into())
        .await;
    assert_eq!(StatusCode::OK, response.status());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions_subscribe(VALID_SUBSCRIPTION_PAYLOAD.into())
        .await;

    let saved = sqlx::query!(
        r#"SELECT email, name, status as "status: SubscriptionStatus" FROM subscriptions WHERE email = $1"#,
        "ursula_le_guin@ztpir.com"
    )
        .fetch_one(&app.connection_pool)
        .await
        .expect("Failed to execute query");

    assert_eq!("ursula_le_guin@ztpir.com", saved.email);
    assert_eq!("le guin", saved.name);
    assert_eq!(SubscriptionStatus::PendingConfirmation, saved.status);
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let app = spawn_app().await;
    let cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40ztpir.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (body, error_description) in cases {
        let response = app.post_subscriptions_subscribe(body.into()).await;

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
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40ztpir.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        let response = app.post_subscriptions_subscribe(body.into()).await;

        assert_eq!(
            StatusCode::BAD_REQUEST,
            response.status(),
            "The API did not return a 400 BAD_REQUEST when the payload was {description}"
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_valid_data() {
    let app = spawn_app().await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions_subscribe(VALID_SUBSCRIPTION_PAYLOAD.into())
        .await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let ConfirmationLinks { html, text } = app.extract_confirmation_links(email_request);
    assert_eq!(html, text);
}

#[tokio::test]
async fn subscribe_returns_500_when_there_is_a_fatal_database_error() {
    let app = spawn_app().await;
    // Sabotage the database
    sqlx::query!("ALTER TABLE subscriptions_confirmation_tokens DROP COLUMN token")
        .execute(&app.connection_pool)
        .await
        .unwrap();

    let response = app
        .post_subscriptions_subscribe(VALID_SUBSCRIPTION_PAYLOAD.into())
        .await;
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}
