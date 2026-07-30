use super::VALID_SUBSCRIPTION_PAYLOAD;
use crate::helpers::spawn_app;
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
        .fetch_optional(&app.connection_pool)
        .await
        .expect("Failed to execute query");

    assert!(saved.is_some());

    let saved = saved.unwrap();

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
    let email_request_body: serde_json::Value =
        serde_json::from_slice(&email_request.body).unwrap();
    let get_link = |s: &str| {
        let links = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect::<Vec<_>>();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let html_link = get_link(&email_request_body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(&email_request_body["TextBody"].as_str().unwrap());
    assert_eq!(html_link, text_link);
}
