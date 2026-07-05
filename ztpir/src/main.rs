use std::net::TcpListener;
use ztpir::run;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    run(TcpListener::bind("127.0.0.1:8080")?)?.await
}
