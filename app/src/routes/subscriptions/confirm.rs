use crate::domain::{SubscriberConfirmationToken, SubscriberId, SubscriptionStatus};
use actix_web::{HttpResponse, web};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: SubscriberConfirmationToken,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(parameters, pool))]
pub async fn confirm(parameters: web::Query<Parameters>, pool: web::Data<PgPool>) -> HttpResponse {
    fn handle_get_subscriber_error(err: sqlx::Error) -> HttpResponse {
        tracing::error!(
            "Failed to retrieve subscriber from confirmation token: {:?}",
            err
        );
        HttpResponse::InternalServerError().finish()
    }
    fn handle_unauthorized_token(
        token: &SubscriberConfirmationToken,
    ) -> impl FnOnce() -> HttpResponse {
        move || -> HttpResponse {
            tracing::warn!("Supplied subscriber confirmation token not found: {token}");
            HttpResponse::Unauthorized().finish()
        }
    }
    fn handle_confirm_subscriber_error(err: sqlx::Error) -> HttpResponse {
        tracing::error!("Failed to confirm subscriber: {:?}", err);
        HttpResponse::InternalServerError().finish()
    }

    async {
        let subscriber_id: SubscriberId =
            get_subscriber_id_from_token(&pool, &parameters.subscription_token)
                .await
                .map_err(handle_get_subscriber_error)?
                .ok_or_else(handle_unauthorized_token(&parameters.subscription_token))?;
        confirm_subscriber(&pool, subscriber_id)
            .await
            .map_err(handle_confirm_subscriber_error)?;

        Ok(HttpResponse::Ok().finish())
    }
    .await
    .unwrap_or_else(|err| err)
}

#[tracing::instrument(name = "Get subscriber_id from token", skip(pool, token))]
async fn get_subscriber_id_from_token(
    pool: &PgPool,
    token: &SubscriberConfirmationToken,
) -> Result<Option<SubscriberId>, sqlx::Error> {
    sqlx::query!(
        r#"SELECT subscriptions_id FROM subscriptions_confirmation_tokens WHERE token = $1"#,
        token
    )
    .fetch_optional(pool)
    .await
    .map_err(|err| {
        tracing::error!("Failed to fetch subscriber_id from token: {:?}", err);
        err
    })
    .map(|maybe_row| maybe_row.and_then(|row| row.subscriptions_id.map(|id| id as SubscriberId)))
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
