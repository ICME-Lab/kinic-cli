use std::process::ExitCode;

use _lib as kinic_cli;
use _lib::agent::extract_keychain_error_code;
use anyhow::Error;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    if let Err(e) = kinic_cli::run().await {
        if let Some(clap_error) = e.downcast_ref::<clap::Error>() {
            let _ = clap_error.print();
            return ExitCode::from(clap_error.exit_code().try_into().unwrap_or(1));
        }
        eprintln!("{}", render_runtime_error(&e));
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

fn render_runtime_error(error: &Error) -> String {
    error
        .chain()
        .map(ToString::to_string)
        .find(|message| extract_keychain_error_code(message).is_some())
        .unwrap_or_else(|| render_error_chain(error))
}

fn render_error_chain(error: &Error) -> String {
    let messages = error.chain().map(ToString::to_string).collect::<Vec<_>>();
    if messages.len() <= 1 {
        return messages
            .into_iter()
            .next()
            .unwrap_or_else(|| error.to_string());
    }

    let mut rendered = messages[0].clone();
    rendered.push_str("\nCaused by:");
    for cause in messages.iter().skip(1) {
        rendered.push_str("\n- ");
        rendered.push_str(cause);
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;

    #[test]
    fn render_runtime_error_prefers_keychain_tagged_cause() {
        let error: anyhow::Result<()> =
            { Err(anyhow::anyhow!("[KEYCHAIN_ACCESS_DENIED] denied")).context("wrapper context") };
        let error = error.unwrap_err();

        assert_eq!(
            render_runtime_error(&error),
            "[KEYCHAIN_ACCESS_DENIED] denied"
        );
    }

    #[test]
    fn render_runtime_error_uses_plain_display_for_non_keychain_errors() {
        let error = anyhow::anyhow!("plain failure");

        assert_eq!(render_runtime_error(&error), "plain failure");
    }

    #[test]
    fn render_runtime_error_includes_non_keychain_cause_chain() {
        let error: anyhow::Result<()> = {
            Err(anyhow::anyhow!("transport backend refused connection"))
                .context("failed to reach local replica")
        };
        let error = error.unwrap_err();

        assert_eq!(
            render_runtime_error(&error),
            "failed to reach local replica\nCaused by:\n- transport backend refused connection"
        );
    }
}
