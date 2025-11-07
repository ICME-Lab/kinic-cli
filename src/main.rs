use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    if let Err(e) = kinic_cli::run().await {
        eprintln!("{e:?}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
