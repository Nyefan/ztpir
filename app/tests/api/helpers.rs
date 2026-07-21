use app::configuration::DatabaseSettings;
use app::email_client::EmailClient;
use app::telemetry::{get_subscriber, init_subscriber};
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use std::sync::LazyLock;
use std::time::Duration;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use uuid::Uuid;

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
}

pub async fn spawn_app() -> TestApp {
    LazyLock::force(&TRACING);
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind listener");
    let port = listener
        .local_addr()
        .expect("listener bound without an address")
        .port();
    let address = format!("http://127.0.0.1:{port}");

    let mut config = app::configuration::get_config().expect("Failed to read configuration.");
    config.database.schema_name = Uuid::now_v7().to_string();
    let connection_pool = configure_database(&config.database).await;

    let sender_email = config
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let base_url = reqwest::Url::parse(&config.email_client.base_url).expect("Invalid base url");
    let authorization_token = config.email_client.authorization_token;
    let email_client = EmailClient::new(
        base_url,
        sender_email,
        authorization_token,
        Duration::from_secs(10),
    );

    let server = app::startup::run_server(listener, connection_pool.clone(), email_client)
        .expect("Failed to spawn server");
    tokio::spawn(server);
    TestApp {
        address,
        connection_pool,
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
