use crate::domain::{
    NewSubscriber, SubscriberConfirmationToken, SubscriberEmail, SubscriberName, SubscriptionStatus,
};
use crate::email_client::EmailClient;
use crate::error::{Error, OnErrorReturn, OnErrorStatus};
use crate::startup::ApplicationBaseUrl;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, web};
use anyhow::Context;
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use sqlx::{PgPool, Postgres, Transaction};
use std::fmt::Debug;
use tracing::instrument;
use uuid::Uuid;

// TODO: mask email and name as SecretStrings - those are also PII and shouldn't be logged except for errors
#[derive(Debug, serde::Deserialize)]
pub(crate) struct FormData {
    email: String,
    name: String,
}
impl TryFrom<FormData> for NewSubscriber {
    type Error = anyhow::Error;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = form.name.parse::<SubscriberName>()?;
        let email = form.email.parse::<SubscriberEmail>()?;
        let confirmation_token = rng()
            .sample_iter(Alphanumeric)
            .map(char::from)
            .take(25)
            .collect();
        Ok(Self {
            name,
            email,
            confirmation_token,
        })
    }
}

#[instrument(
    name = "New subscription request received",
    skip(form, pool, email_client, application_base_url),
    fields(email = %form.email, name = %form.name)
)]
pub(crate) async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    application_base_url: web::Data<ApplicationBaseUrl>,
) -> Result<HttpResponse, Error> {
    let subscriber = form
        .into_inner()
        .try_into()
        .on_error_status(StatusCode::BAD_REQUEST)?;

    // should the transaction not commit until the email is sent?  long transaction = bad, but
    // idempotency is not guaranteed by the compiler - how would we even represent that???
    {
        let mut transaction = pool.begin().await.on_error_return(
            StatusCode::SERVICE_UNAVAILABLE,
            "Failed to acquire a database connection from the pool.  Try again later.",
        )?;
        let subscriber_id = insert_subscriber(&mut transaction, &subscriber)
            .await
            .on_error_return(
                StatusCode::INTERNAL_SERVER_ERROR,
                "A database error was encountered while trying to create a new subscription.",
            )?;
        insert_subscriber_confirmation_token(
            &mut transaction,
            &subscriber_id,
            &subscriber.confirmation_token,
        )
            .await
            .on_error_return(
                StatusCode::INTERNAL_SERVER_ERROR,
                "A database error was encountered while trying to create a subscription confirmation token.",
            )?;
        transaction
            .commit()
            .await
            .on_error_return(StatusCode::BAD_GATEWAY, "Failed to commit transaction.")?;
    }

    send_confirmation_email(&email_client, &subscriber, &application_base_url)
        .await
        .on_error_status(StatusCode::BAD_GATEWAY)?;

    Ok(HttpResponse::Ok().finish())
}

#[instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let id = sqlx::query!(
        r#"
            INSERT INTO subscriptions(email, name, status)
            VALUES($1, $2, $3::subscription_status)
            RETURNING id
        "#,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        SubscriptionStatus::PendingConfirmation as SubscriptionStatus
    )
    .fetch_one(&mut **transaction)
    .await
    .map(|result| result.id)?;
    Ok(id)
}

async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
    base_url: &ApplicationBaseUrl,
) -> Result<(), anyhow::Error> {
    let NewSubscriber {
        name,
        email,
        confirmation_token,
    } = subscriber;
    let confirmation_link =
        format!("{base_url}/subscriptions/confirm?subscription_token={confirmation_token}");
    let subject = format!("Welcome {}!", name);
    let html_body = format!(
        "Welcome to our newsletter!<br />\
                        Click <a href=\"{confirmation_link}\">here</a> to confirm your subscription."
    );
    let text_body = format!(
        "Welcome to our newsletter!\nVisit {confirmation_link} to confirm your subscription."
    );
    email_client
        .send_email(email, &subject, &html_body, &text_body)
        .await
        .context("Failed to send confirmation email.")?;
    Ok(())
}

async fn insert_subscriber_confirmation_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: &Uuid,
    confirmation_token: &SubscriberConfirmationToken,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
            INSERT INTO subscriptions_confirmation_tokens (subscriptions_id, token)
            VALUES ($1, $2)
        "#,
        subscriber_id,
        confirmation_token
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

// TODO: test the actual behavior of subscribe (i.e. that it inserts into the db, etc.)
