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
    fn bad_request(err: String) -> HttpResponse {
        tracing::warn!("Failed to parse form data: {:?}", err);
        HttpResponse::BadRequest().body(err)
    }
    fn database_error(err: sqlx::Error) -> HttpResponse {
        tracing::error!("Failed to insert into subscriptions: {:?}", err);
        HttpResponse::InternalServerError().finish()
    }
    fn email_client_error(err: String) -> HttpResponse {
        tracing::error!("Failed to send confirmation email: {:?}", err);
        HttpResponse::InternalServerError().finish()
    }

    (async || {
        let subscriber: NewSubscriber = form.into_inner().try_into().map_err(bad_request)?;
        insert_subscriber(&connection_pool, &subscriber)
            .await
            .map_err(database_error)?;
        send_confirmation_email(&email_client, &subscriber)
            .await
            .map_err(email_client_error)?;

        Ok(HttpResponse::Ok().finish())
    })()
    .await
    .unwrap_or_else(|err| err)
}

async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
) -> Result<(), String> {
    let confirmation_link = "https://ztpir.nyefan.org/api/subscriptions/confirm";
    let subject = format!("Welcome {}!", &subscriber.name);
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription."
    );
    let text_body = format!(
        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription."
    );
    email_client
        .send_email(&subscriber.email, &subject, &html_body, &text_body)
        .await
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
