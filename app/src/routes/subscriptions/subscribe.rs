use crate::domain::{
    NewSubscriber, SubscriberConfirmationToken, SubscriberEmail, SubscriberName, SubscriptionStatus,
};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use sqlx::{PgPool, Postgres, Transaction};
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use tracing::instrument;
use uuid::Uuid;

// TODO: mask email and name as SecretStrings - those are also PII and shouldn't be logged except for errors
#[derive(Debug, serde::Deserialize)]
pub(crate) struct FormData {
    email: String,
    name: String,
}
impl TryFrom<FormData> for NewSubscriber {
    type Error = SubscribeError;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name =
            SubscriberName::parse(form.name).map_err(SubscribeError::InvalidInputReceived)?;
        let email =
            SubscriberEmail::parse(form.email).map_err(SubscribeError::InvalidInputReceived)?;
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

pub static DATABASE_POOL_EXHAUSTED_ERROR_MESSAGE: &str =
    "Failed to acquire a database connection from the pool.";
pub static DATABASE_TRANSACTION_COMMIT_ERROR_MESSAGE: &str = "Failed to commit transaction.";
pub static INSERT_SUBSCRIBER_ERROR_MESSAGE: &str =
    "A database error was encountered while trying to create a new subscription.";
pub static INSERT_SUBSCRIBER_TOKEN_ERROR_MESSAGE: &str =
    "A database error was encountered while trying to create a subscription confirmation token.";
pub static SEND_CONFIRMATION_EMAIL_ERROR_MESSAGE: &str = "Failed to send confirmation email.";
pub static VALIDATION_ERROR_MESSAGE: &str = "Invalid input received.";

pub enum SubscribeError {
    ConfirmationEmailNotSent(String /* reqwest::Error */),
    DatabasePoolExhausted(sqlx::Error),
    DatabaseTransactionNotCommitted(sqlx::Error),
    InvalidInputReceived(String),
    SubscriberNotInserted(sqlx::Error),
    SubscriberTokenNotInserted(sqlx::Error),
}
impl Display for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let reason = match self {
            Self::ConfirmationEmailNotSent(_) => SEND_CONFIRMATION_EMAIL_ERROR_MESSAGE,
            Self::DatabasePoolExhausted(_) => DATABASE_POOL_EXHAUSTED_ERROR_MESSAGE,
            Self::DatabaseTransactionNotCommitted(_) => DATABASE_TRANSACTION_COMMIT_ERROR_MESSAGE,
            Self::InvalidInputReceived(_) => VALIDATION_ERROR_MESSAGE,
            Self::SubscriberNotInserted(_) => INSERT_SUBSCRIBER_ERROR_MESSAGE,
            Self::SubscriberTokenNotInserted(_) => INSERT_SUBSCRIBER_TOKEN_ERROR_MESSAGE,
        };
        write!(f, "{reason}")
    }
}
impl Debug for SubscribeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        super::error_chain_fmt(self, f)
    }
}
impl Error for SubscribeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SubscribeError::ConfirmationEmailNotSent(_) => None, // TODO: reqwest::Error
            SubscribeError::DatabasePoolExhausted(e) => Some(e),
            SubscribeError::DatabaseTransactionNotCommitted(e) => Some(e),
            SubscribeError::InvalidInputReceived(_) => None,
            SubscribeError::SubscriberNotInserted(e) => Some(e),
            SubscribeError::SubscriberTokenNotInserted(e) => Some(e),
        }
    }
}
impl ResponseError for SubscribeError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ConfirmationEmailNotSent(_) => StatusCode::BAD_GATEWAY,
            Self::DatabasePoolExhausted(_) => StatusCode::SERVICE_UNAVAILABLE,
            Self::DatabaseTransactionNotCommitted(_) => StatusCode::BAD_GATEWAY,
            Self::InvalidInputReceived(_) => StatusCode::BAD_REQUEST,
            Self::SubscriberNotInserted(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::SubscriberTokenNotInserted(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
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
) -> Result<HttpResponse, SubscribeError> {
    let subscriber = form.into_inner().try_into()?;

    // should the transaction not commit until the email is sent?  long transaction = bad, but
    // idempotency is not guaranteed by the compiler - how would we even represent that???
    {
        let mut transaction = pool
            .begin()
            .await
            .map_err(SubscribeError::DatabasePoolExhausted)?;
        let subscriber_id = insert_subscriber(&mut transaction, &subscriber).await?;
        insert_subscriber_confirmation_token(
            &mut transaction,
            &subscriber_id,
            &subscriber.confirmation_token,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(SubscribeError::DatabaseTransactionNotCommitted)?;
    }

    send_confirmation_email(&email_client, &subscriber, &application_base_url).await?;

    Ok(HttpResponse::Ok().finish())
}

#[instrument(
    name = "Saving new subscriber details in the database",
    skip(transaction, new_subscriber)
)]
async fn insert_subscriber(
    transaction: &mut Transaction<'_, Postgres>,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, SubscribeError> {
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
    .map(|result| result.id)
    .map_err(SubscribeError::SubscriberNotInserted)?;
    Ok(id)
}

async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
    base_url: &ApplicationBaseUrl,
) -> Result<(), SubscribeError> {
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
        .map_err(SubscribeError::ConfirmationEmailNotSent)?;
    Ok(())
}

async fn insert_subscriber_confirmation_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: &Uuid,
    confirmation_token: &SubscriberConfirmationToken,
) -> Result<(), SubscribeError> {
    sqlx::query!(
        r#"
            INSERT INTO subscriptions_confirmation_tokens (subscriptions_id, token)
            VALUES ($1, $2)
        "#,
        subscriber_id,
        confirmation_token
    )
    .execute(&mut **transaction)
    .await
    .map_err(SubscribeError::SubscriberTokenNotInserted)?;
    Ok(())
}

// TODO: test the actual behavior of subscribe (i.e. that it inserts into the db, etc.)
