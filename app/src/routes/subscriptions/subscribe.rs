use crate::domain::{
    NewSubscriber, SubscriberConfirmationToken, SubscriberEmail, SubscriberName, SubscriptionStatus,
};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;
use actix_web::{HttpResponse, web};
use rand::distr::Alphanumeric;
use rand::{RngExt, rng};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

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
    skip(form, connection_pool, email_client, base_url),
    fields(email = %form.email, name = %form.name)
)]
pub(crate) async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> HttpResponse {
    fn bad_request(err: String) -> HttpResponse {
        tracing::warn!("Failed to parse form data: {:?}", err);
        HttpResponse::BadRequest().body(err)
    }
    fn insert_subscriber_error(err: sqlx::Error) -> HttpResponse {
        tracing::error!("Failed to insert into subscriptions: {:?}", err);
        HttpResponse::InternalServerError().finish()
    }
    fn insert_subscriber_confirmation_token_error(err: sqlx::Error) -> HttpResponse {
        tracing::error!("Failed to insert subscriber token: {:?}", err);
        HttpResponse::InternalServerError().finish()
    }
    fn email_client_error(err: String) -> HttpResponse {
        tracing::error!("Failed to send confirmation email: {:?}", err);
        HttpResponse::InternalServerError().finish()
    }

    async {
        let subscriber: NewSubscriber = form.into_inner().try_into().map_err(bad_request)?;
        let subscriber_id = insert_subscriber(&connection_pool, &subscriber)
            .await
            .map_err(insert_subscriber_error)?;
        insert_subscriber_confirmation_token(
            &connection_pool,
            &subscriber_id,
            &subscriber.confirmation_token,
        )
        .await
        .map_err(insert_subscriber_confirmation_token_error)?;
        send_confirmation_email(&email_client, &subscriber, &base_url)
            .await
            .map_err(email_client_error)?;

        Ok(HttpResponse::Ok().finish())
    }
    .await
    .unwrap_or_else(|err| err)
}

async fn insert_subscriber_confirmation_token(
    pool: &PgPool,
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
    .execute(pool)
    .await
    .map(|_| {
        tracing::debug!("Inserted subscription_token: {confirmation_token}");
    })
    .map_err(|e| {
        tracing::error!("Failed to insert subscription_token: {:?}", e);
        e
    })
}

async fn send_confirmation_email(
    email_client: &EmailClient,
    subscriber: &NewSubscriber,
    base_url: &ApplicationBaseUrl,
) -> Result<(), String> {
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
}

#[instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query!(
        r#"
            INSERT INTO subscriptions(email, name, status)
            VALUES($1, $2, $3::subscription_status)
            RETURNING id
        "#,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        SubscriptionStatus::PendingConfirmation as SubscriptionStatus
    )
    .fetch_one(pool)
    .await
    .map(|result| result.id)
    .map_err(|e| {
        tracing::error!("Failed to insert subscriber: {:?}", e);
        e
    })
}

// TODO: test the actual behavior of subscribe (i.e. that it inserts into the db, etc.)
