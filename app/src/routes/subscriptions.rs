use actix_web::{web, HttpResponse};
use sqlx::PgPool;
use tracing::instrument;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName, SubscriptionStatus};
use crate::email_client::EmailClient;

// TODO: mask email and name as SecretStrings - those are also PII and shouldn't be logged except for errors
#[derive(Debug, serde::Deserialize)]
pub(crate) struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name)?;
        let email = SubscriberEmail::parse(form.email)?;
        Ok(Self { name, email })
    }
}

#[instrument(
    name = "New subscription request received",
    skip(form, connection_pool, email_client),
    fields(email = %form.email, name = %form.name)
)]
pub(crate) async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> HttpResponse {
    let confirmation_link = "https://ztpir.nyefan.org/api/subscriptions/confirm";
    match form.into_inner().try_into() {
        Err(e) => HttpResponse::BadRequest().body(e),
        Ok(subscriber) => match insert_subscriber(&connection_pool, &subscriber).await {
            Err(e) => {
                tracing::error!("Failed to insert into subscriptions: {:?}", e);
                HttpResponse::InternalServerError().finish()
            }
            Ok(_) => match email_client
                .send_email(
                    subscriber.email,
                    "Welcome!",
                    &format!(
                        "Welcome to our newsletter!<br />\
                        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription."
                    ),
                    &format!(
                        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription."
                    ),
                )
                .await
            {
                Err(e) => {
                    tracing::error!("Failed to send email: {:?}", e);
                    HttpResponse::InternalServerError().finish()
                }
                Ok(_) => HttpResponse::Ok().finish(),
            },
        },
    }
}

#[instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            INSERT INTO subscriptions(email, name, status)
            VALUES($1, $2, $3::subscription_status)
        "#,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        SubscriptionStatus::PendingConfirmation as SubscriptionStatus
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to insert subscriber: {:?}", e);
        e
    })?;
    Ok(())
}

// TODO: test the actual behavior of subscribe (i.e. that it inserts into the db, etc.)
