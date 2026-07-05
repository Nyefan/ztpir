use actix_web::dev::Server;
use actix_web::{App, HttpResponse, HttpServer, web};

pub fn run() -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().route("/healthz", web::get().to(health_check)))
        .bind("127.0.0.1:8080")?
        .run();
    Ok(server)
}

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn health_check_returns_ok() {
        let response = health_check().await;
        assert!(response.status().is_success());
    }
}
