#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    ztpir::startup::startup().await
}
