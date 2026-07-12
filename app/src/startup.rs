use crate::configuration::get_config;
use crate::{routes, telemetry};
use actix_web::{App, HttpServer, dev, web};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use std::net::TcpListener;
use tracing_actix_web::TracingLogger;

pub async fn startup() -> Result<(), std::io::Error> {
    telemetry::init_subscriber(telemetry::get_subscriber(
        "ztpir".into(),
        "info".into(),
        std::io::stdout,
    ));
    let configuration = get_config().expect("Failed to read configuration");
    tracing::info!("Starting server with configuration: {:?}", configuration);
    let address = format!(
        "{}:{}",
        configuration.application.interface, configuration.application.port
    );
    let listener = TcpListener::bind(address)?;
    let connection =
        PgPoolOptions::new().connect_lazy_with(configuration.database.connect_options());
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
