use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::TcpListener;
use std::time::Duration;
use tracing_actix_web::TracingLogger;

pub fn get_connection_pool(config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().connect_lazy_with(config.connect_options())
}

pub struct Application {
    pub port: u16,
    server: Server,
}

impl Application {
    pub async fn build(config: Settings) -> Result<Self, std::io::Error> {
        let connection_pool =
            PgPoolOptions::new().connect_lazy_with(config.database.connect_options());

        let sender_email = config
            .email_client
            .sender()
            .expect("Invalid sender email address");
        let base_url =
            reqwest::Url::parse(&config.email_client.base_url).expect("Invalid base url");
        let authorization_token = config.email_client.authorization_token;
        let email_client_timeout =
            Duration::from_millis(config.email_client.timeout_milliseconds.into());
        let email_client = EmailClient::new(
            base_url,
            sender_email,
            authorization_token,
            email_client_timeout,
        );

        let address = format!(
            "{}:{}",
            config.application.interface, config.application.port
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr()?.port();
        let server = Self::spawn_server(listener, connection_pool, email_client)?;

        Ok(Self { port, server })
    }

    fn spawn_server(
        listener: TcpListener,
        connection_pool: PgPool,
        email_client: EmailClient,
    ) -> Result<Server, std::io::Error> {
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

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
