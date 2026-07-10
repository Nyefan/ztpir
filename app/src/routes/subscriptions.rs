use actix_web::{HttpResponse, web};

#[derive(serde::Deserialize)]
pub(crate) struct FormData {
    email: String,
    name: String,
}

pub(crate) async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<sqlx::PgPool>,
) -> HttpResponse {
    tracing::info!(
        "New subscription request received",
    );

    match sqlx::query!(
        r#"
            INSERT INTO subscriptions(email, name)
            VALUES($1, $2)
        "#,
        form.email,
        form.name
    )
    .execute(connection_pool.get_ref())
    .await
    {
        Ok(_) => {
            tracing::info!(
                "New subscriber details have been saved",
            );
            HttpResponse::Ok().finish()
        }
        Err(e) => {
            tracing::error!(
                "Failed to insert into subscriptions: {:?}",
                e
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}

// TODO: test the actual behavior of subscribe (i.e. that it inserts into the db, etc.)
