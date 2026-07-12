use actix_web::{HttpResponse, web};
use sqlx::PgPool;
use tracing::instrument;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

// TODO: mask email and name - those are also PII and shouldn't be logged except for errors
#[derive(Debug, serde::Deserialize)]
pub(crate) struct FormData {
    email: String,
    name: String,
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
    let new_subscriber = NewSubscriber {
        name: SubscriberName::parse(form.name.clone()).unwrap(),
        email: SubscriberEmail::parse(form.email.clone()).unwrap(),
    };
    match insert_subscriber(&connection_pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) => {
            tracing::error!("Failed to insert into subscriptions: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
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
