use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use tracing::instrument;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

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
    skip(form, connection_pool),
    fields(email = %form.email, name = %form.name)
)]
pub(crate) async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
) -> HttpResponse {
    match form.into_inner().try_into() {
        Err(e) => HttpResponse::BadRequest().body(e),
        Ok(subscriber) => match insert_subscriber(&connection_pool, &subscriber).await {
            Err(e) => {
                tracing::error!("Failed to insert into subscriptions: {:?}", e);
                HttpResponse::InternalServerError().finish()
            }
            Ok(_) => HttpResponse::Ok().finish(),
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
            INSERT INTO subscriptions(email, name)
            VALUES($1, $2)
        "#,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
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
