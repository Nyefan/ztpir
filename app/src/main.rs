use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::layer::SubscriberExt;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing::subscriber::set_global_default(
        tracing_subscriber::Registry::default().with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(tracing_subscriber::EnvFilter::new("info")),
        ).with(JsonStorageLayer)
            .with(BunyanFormattingLayer::new("ztpir".into(), std::io::stdout)),
    ).expect("Failed to set tracing subscriber");
    app::startup::startup().await
}
