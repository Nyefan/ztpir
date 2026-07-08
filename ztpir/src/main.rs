use std::net::TcpListener;
use ztpir::startup::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    run(TcpListener::bind("127.0.0.1:8080")?)?.await
}
