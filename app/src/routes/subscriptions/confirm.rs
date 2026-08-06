use crate::domain::{SubscriberConfirmationToken, SubscriberId, SubscriptionStatus};
use crate::error::{Error, OnErrorReturn, WhenNoneReturn};
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: SubscriberConfirmationToken,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(
    parameters: web::Query<Parameters>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let token = &parameters.subscription_token;
    let subscriber_id = get_subscriber_id_from_token(&pool, token)
        .await
        .on_error_return(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to fetch subscriber_id from token",
        )?
        .when_none_return(
            StatusCode::UNAUTHORIZED,
            "Supplied subscriber confirmation token not found: {token}",
        )?;
    confirm_subscriber(&pool, subscriber_id)
        .await
        .on_error_return(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to confirm subscriber",
        )?;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    token: &SubscriberConfirmationToken,
) -> Result<Option<SubscriberId>, sqlx::Error> {
    let id = sqlx::query!(
        r#"SELECT subscriptions_id FROM subscriptions_confirmation_tokens WHERE token = $1"#,
        token
    )
    .fetch_optional(pool)
    .await?
    .and_then(|row| row.subscriptions_id.map(|id| id as SubscriberId));
    Ok(id)
}

#[tracing::instrument(name = "Mark subscriber as confirmed", skip(subscriber_id, pool))]
async fn confirm_subscriber(pool: &PgPool, subscriber_id: SubscriberId) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE subscriptions set status = $1::subscription_status WHERE id = $2"#,
        SubscriptionStatus::Confirmed as SubscriptionStatus,
        subscriber_id
    )
    .execute(pool)
    .await?;
    Ok(())
}
