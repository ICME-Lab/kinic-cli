use std::process::ExitCode;

use _lib as kinic_cli;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    if let Err(e) = kinic_cli::run().await {
        if let Some(clap_error) = e.downcast_ref::<clap::Error>() {
            let _ = clap_error.print();
            return ExitCode::from(clap_error.exit_code().try_into().unwrap_or(1));
        }
        eprintln!("{e:?}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
