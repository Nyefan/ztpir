use crate::configuration::get_config;
use crate::routes;
use actix_web::{App, HttpServer, dev, web};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub async fn startup() -> Result<(), std::io::Error> {
    let configuration = get_config().expect("Failed to read configuration");
    println!("{:?}", configuration);
    let address = format!("127.0.0.1:{}", configuration.application_port);
    let listener = TcpListener::bind(address)?;
    let connection = PgPool::connect(&configuration.database.connection_string())
        .await
        .expect("Failed to connect to Postgres");
    run_server(listener, connection)?.await
}

pub fn run_server(
    listener: TcpListener,
    connection_pool: PgPool,
) -> Result<dev::Server, std::io::Error> {
    let connection_pool = web::Data::new(connection_pool);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/healthz", web::get().to(routes::health_check))
            .route("/subscriptions", web::post().to(routes::subscribe))
            .app_data(connection_pool.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
