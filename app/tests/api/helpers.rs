use app::configuration::{DatabaseSettings, get_config};
use app::startup::{Application, get_connection_pool};
use app::telemetry::{get_subscriber, init_subscriber};
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use uuid::Uuid;
use wiremock::MockServer;

static TRACING: LazyLock<()> = LazyLock::new(|| {
    let sink = std::env::var("TEST_LOG")
        .map(|_| BoxMakeWriter::new(std::io::stdout))
        .unwrap_or(BoxMakeWriter::new(std::io::sink));
    let subscriber = get_subscriber("test".into(), "debug".into(), sink);
    init_subscriber(subscriber);
});

pub struct TestApp {
    pub(crate) address: String,
    pub(crate) connection_pool: PgPool,
    pub(crate) email_server: MockServer,
    pub(crate) port: u16,
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions_subscribe(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions/subscribe", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub fn extract_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect::<Vec<_>>();
            assert_eq!(links.len(), 1);
            let mut confirmation_link = links[0]
                .as_str()
                .to_owned()
                .parse::<reqwest::Url>()
                .unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "localhost");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let text = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, text }
    }
}

pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);

    let email_server = MockServer::start().await;

    let config = {
        let mut config = get_config().expect("Failed to read configuration.");
        config.database.schema_name = Uuid::now_v7().to_string();
        config.application.port = 0;
        config.email_client.base_url = email_server.uri();
        config
    };

    configure_database(&config.database).await;

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build server");
    let port = app.port();
    let address = format!("http://{}:{}", config.application.interface, port);

    tokio::spawn(app.run_until_stopped());
    TestApp {
        address,
        connection_pool: get_connection_pool(&config.database),
        email_server,
        port,
    }
}

async fn configure_database(app_database_settings: &DatabaseSettings) -> PgPool {
    let pg_database_settings = DatabaseSettings {
        schema_name: "postgres".to_string(),
        // username: "postgres".to_string(),
        // password: "password".to_string(),
        ..app_database_settings.clone()
    };
    PgConnection::connect_with(&pg_database_settings.connect_options())
        .await
        .expect("Failed to connect to Postgres")
        .execute(AssertSqlSafe(format!(
            r#"CREATE DATABASE "{}";"#,
            app_database_settings.schema_name
        )))
        .await
        .expect("Failed to create test schema");

    let connection_pool = PgPool::connect_with(app_database_settings.connect_options())
        .await
        .expect("Failed to connect to database");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}
