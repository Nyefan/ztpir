use app::configuration::get_config;
pub use app::startup::Application;
use app::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let tracing_subscriber =
        get_subscriber("ztpir".to_string(), "info".to_string(), std::io::stdout);
    init_subscriber(tracing_subscriber);

    let config = get_config().expect("Failed to read configuration");
    let app = Application::build(config)
        .await
        .expect("Failed to build server");
    app.run_until_stopped().await?;
    Ok(())
}
