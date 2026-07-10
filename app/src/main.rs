#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    app::startup::startup().await
}
