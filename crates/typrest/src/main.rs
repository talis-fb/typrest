use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    typrest::run().await
}
