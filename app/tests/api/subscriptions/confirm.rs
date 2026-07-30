use super::VALID_SUBSCRIPTION_PAYLOAD;
use crate::helpers::{ConfirmationLinks, spawn_app};
use app::domain::SubscriptionStatus;
use reqwest::StatusCode;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_403() {
    let app = spawn_app().await;

    let response = reqwest::get(&format!("{}/subscriptions/confirm", app.address))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST)
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions_subscribe(VALID_SUBSCRIPTION_PAYLOAD.into())
        .await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let ConfirmationLinks {
        html: confirmation_link,
        ..
    } = app.extract_confirmation_links(email_request);
    assert_eq!(confirmation_link.host_str().unwrap(), "localhost");

    let response = reqwest::get(confirmation_link).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn clicking_on_the_confirmation_link_confirms_a_subscriber() {
    let app = spawn_app().await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(StatusCode::OK))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions_subscribe(VALID_SUBSCRIPTION_PAYLOAD.into())
        .await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let ConfirmationLinks {
        html: confirmation_link,
        ..
    } = app.extract_confirmation_links(email_request);

    reqwest::get(confirmation_link)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let saved = sqlx::query!(
        r#"SELECT email, name, status as "status: SubscriptionStatus" FROM subscriptions"#
    )
    .fetch_one(&app.connection_pool)
    .await
    .expect("Failed to execute query.");

    assert_eq!("ursula_le_guin@ztpir.com", saved.email);
    assert_eq!("le guin", saved.name);
    assert_eq!(SubscriptionStatus::Confirmed, saved.status);
}
