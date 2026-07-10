use actix_web::HttpResponse;
use tracing::instrument;

#[instrument(name = "Health check endpoint", level = "trace")]
pub(crate) async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}
