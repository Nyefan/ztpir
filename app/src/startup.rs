use crate::configuration::get_config;
use crate::email_client::EmailClient;
use crate::{routes, telemetry};
use actix_web::{dev, web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use std::time::Duration;
use tracing_actix_web::TracingLogger;

pub async fn startup() -> Result<(), std::io::Error> {
    telemetry::init_subscriber(telemetry::get_subscriber(
        "ztpir".into(),
        "info".into(),
        std::io::stdout,
    ));
    let config = get_config().expect("Failed to read configuration");
    tracing::info!("Starting server with configuration: {:?}", config);

    let address = format!(
        "{}:{}",
        config.application.interface, config.application.port
    );
    let listener = TcpListener::bind(address)?;

    let connection_pool = PgPoolOptions::new().connect_lazy_with(config.database.connect_options());

    let sender_email = config
        .email_client
        .sender()
        .expect("Invalid sender email address");
    let base_url = reqwest::Url::parse(&config.email_client.base_url).expect("Invalid base url");
    let authorization_token = config.email_client.authorization_token;
    let email_client_timeout =
        Duration::from_millis(config.email_client.timeout_milliseconds.into());
    let email_client = EmailClient::new(
        base_url,
        sender_email,
        authorization_token,
        email_client_timeout,
    );

    run_server(listener, connection_pool, email_client)?.await
}

pub fn run_server(
    listener: TcpListener,
    connection_pool: PgPool,
    email_client: EmailClient,
) -> Result<dev::Server, std::io::Error> {
    let connection_pool = web::Data::new(connection_pool);
    let email_client = web::Data::new(email_client);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/healthz", web::get().to(routes::health_check))
            .route("/subscriptions", web::post().to(routes::subscribe))
            .app_data(connection_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(listener)?
    .run();
    Ok(server)
}
