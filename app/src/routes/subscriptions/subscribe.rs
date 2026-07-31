use std::error::Error;
use crate::domain::{
    NewSubscriber, SubscriberConfirmationToken, SubscriberEmail, SubscriberName, SubscriptionStatus,
};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::error::ErrorInternalServerError;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, ResponseError, web};
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use sqlx::{PgPool, Postgres, Transaction};
use std::fmt::{Display, Formatter};
use tracing::instrument;
use uuid::Uuid;

// TODO: mask email and name as SecretStrings - those are also PII and shouldn't be logged except for errors
#[derive(Debug, serde::Deserialize)]
pub(crate) struct FormData {
    email: String,
    name: String,
}
impl TryFrom<FormData> for NewSubscriber {
    type Error = actix_web::error::Error;

    fn try_from(form: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form.name).map_err(actix_web::error::ErrorBadRequest)?;
        let email =
            SubscriberEmail::parse(form.email).map_err(actix_web::error::ErrorBadRequest)?;
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

#[derive(Debug)]
pub struct InsertSubscriberError(sqlx::Error);
impl InsertSubscriberError {
    pub const ERROR_MESSAGE: &str =
        "A database error was encountered while trying to create a new subscription.";
}
impl Display for InsertSubscriberError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::ERROR_MESSAGE)
    }
}
impl ResponseError for InsertSubscriberError {}
impl From<sqlx::Error> for InsertSubscriberError {
    fn from(err: sqlx::Error) -> Self {
        InsertSubscriberError(err)
    }
}
impl Error for InsertSubscriberError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Debug)]
pub struct InsertSubscriberTokenError(sqlx::Error);
impl InsertSubscriberTokenError {
    pub const ERROR_MESSAGE: &str = "A database error was encountered while trying to create a subscription confirmation token.";
}
impl Display for InsertSubscriberTokenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::ERROR_MESSAGE)
    }
}
impl ResponseError for InsertSubscriberTokenError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_GATEWAY
    }
}
impl From<sqlx::Error> for InsertSubscriberTokenError {
    fn from(err: sqlx::Error) -> Self {
        InsertSubscriberTokenError(err)
    }
}
impl Error for InsertSubscriberTokenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}

#[derive(Debug)]
#[expect(dead_code)] // we use this exclusively to print useful error logs, but debug is ignored by dead code analysis
pub struct SendConfirmationEmailError(String);
impl SendConfirmationEmailError {
    pub const ERROR_MESSAGE: &str = "A database error was encountered while trying to create a subscription confirmation token.";
}
impl Display for SendConfirmationEmailError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::ERROR_MESSAGE)
    }
}
impl ResponseError for SendConfirmationEmailError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_GATEWAY
    }
}
impl From<String> for SendConfirmationEmailError {
    fn from(err: String) -> Self {
        SendConfirmationEmailError(err)
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
) -> Result<HttpResponse, actix_web::Error> {
    let subscriber = form.into_inner().try_into()?;

    // should the transaction not commit until the email is sent?  long transaction = bad, but
    // idempotency is not guaranteed by the compiler - how would we even represent that???
    {
        let mut transaction = pool.begin().await.map_err(ErrorInternalServerError)?;
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
            .map_err(ErrorInternalServerError)?;
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
) -> Result<Uuid, InsertSubscriberError> {
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

async fn insert_subscriber_confirmation_token(
    transaction: &mut Transaction<'_, Postgres>,
    subscriber_id: &Uuid,
    confirmation_token: &SubscriberConfirmationToken,
) -> Result<(), InsertSubscriberTokenError> {
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

async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
    base_url: &ApplicationBaseUrl,
) -> Result<(), SendConfirmationEmailError> {
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
        .await?;
    Ok(())
}

// TODO: test the actual behavior of subscribe (i.e. that it inserts into the db, etc.)
