use app::configuration::{get_config, DatabaseSettings};
use app::startup::{get_connection_pool, Application};
use app::telemetry::{get_subscriber, init_subscriber};
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection, PgPool};
use std::sync::LazyLock;
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

    let config = {
        let mut config = get_config().expect("Failed to read configuration.");
        config.database.schema_name = Uuid::now_v7().to_string();
        config.application.port = 0;
        config
    };

    configure_database(&config.database).await;

    let app = Application::build(config.clone())
        .await
        .expect("Failed to build server");
    let address = format!("http://{}:{}", config.application.interface, app.port);

    tokio::spawn(app.run_until_stopped());
    TestApp {
        address,
        connection_pool: get_connection_pool(&config.database),
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
