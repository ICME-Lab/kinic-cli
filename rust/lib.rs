pub mod agent;
#[path = "cli_defs.rs"]
pub mod cli;
pub(crate) mod clients;
mod commands;
pub(crate) mod create_domain;
mod embedding;
pub(crate) mod identity_store;
pub(crate) mod insert_service;
mod ledger;
pub(crate) mod memory_client_builder;
mod operation_timeout;
pub(crate) mod preferences;
mod prompt_utils;
#[cfg(feature = "python-bindings")]
mod python;
pub(crate) mod shared;
pub mod tui;

use anyhow::{Result, anyhow};
use clap::{CommandFactory, Parser, error::ErrorKind};
use std::path::PathBuf;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt;

use crate::{
    agent::AgentFactory,
    cli::Cli,
    commands::{CommandContext, capabilities, prefs, run_command},
};

pub(crate) const KEYRING_IDENTITY_REQUIRED_MESSAGE: &str =
    "--identity is required unless --ii is set";

#[cfg(feature = "python-bindings")]
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyModule,
    wrap_pyfunction,
};
#[cfg(feature = "python-bindings")]
use tokio::runtime::Runtime;

pub(crate) const TUI_IDENTITY_REQUIRED_MESSAGE: &str = "--identity is required for the Kinic TUI";
pub(crate) const TUI_II_UNSUPPORTED_MESSAGE: &str =
    "Internet Identity is not supported for the Kinic TUI yet";

fn log_level_for_verbose(verbose: u8) -> LevelFilter {
    match verbose {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        2 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    }
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    validate_tui_cli_args(&cli)?;
    validate_keyring_identity(&cli)?;

    let max = log_level_for_verbose(cli.global.verbose);

    fmt()
        .with_max_level(max)
        .without_time()
        .with_writer(std::io::stderr)
        .try_init()
        .ok();

    if matches!(&cli.command, cli::Command::Tui(_)) {
        return tui::run(&cli.global);
    }

    match cli.command {
        cli::Command::Capabilities(args) => capabilities::handle(args),
        cli::Command::Prefs(args) => prefs::handle(args, &cli.global).await,
        command => {
            if cli.global.ii
                && matches!(
                    &command,
                    cli::Command::Create(_) | cli::Command::Balance(_) | cli::Command::Transfer(_)
                )
                && !cfg!(feature = "experimental")
            {
                anyhow::bail!(
                    "For security reasons, using a locally hosted origin Internet Identity is not recommended for commands involving asset transfers."
                );
            }

            let (agent_factory, identity_path) = if matches!(&command, cli::Command::Login(_)) {
                let identity_path = Some(resolve_identity_path(&cli.global)?);
                (
                    AgentFactory::new(cli.global.ic, String::new()),
                    identity_path,
                )
            } else {
                build_cli_command_context(&cli.global)?
            };

            let context = CommandContext {
                agent_factory,
                identity_path,
            };

            run_command(command, context).await
        }
    }
}

fn validate_tui_cli_args(cli: &Cli) -> Result<()> {
    if !matches!(&cli.command, cli::Command::Tui(_)) {
        return Ok(());
    }

    if cli.global.ii {
        let mut command = Cli::command();
        let clap_error = command
            .error(ErrorKind::ArgumentConflict, TUI_II_UNSUPPORTED_MESSAGE)
            .with_cmd(&command);
        return Err(clap_error.into());
    }

    if cli.global.identity.is_none() {
        let mut command = Cli::command();
        let clap_error = command
            .error(
                ErrorKind::MissingRequiredArgument,
                TUI_IDENTITY_REQUIRED_MESSAGE,
            )
            .with_cmd(&command);
        return Err(clap_error.into());
    }

    Ok(())
}

fn validate_keyring_identity(cli: &Cli) -> Result<()> {
    if cli.global.ii {
        return Ok(());
    }
    if matches!(&cli.command, cli::Command::Login(_)) {
        return Ok(());
    }
    if matches!(&cli.command, cli::Command::Tui(_)) {
        return Ok(());
    }
    if matches!(&cli.command, cli::Command::Capabilities(_)) {
        return Ok(());
    }
    if matches!(&cli.command, cli::Command::Prefs(_)) {
        return Ok(());
    }
    if cli.global.identity.is_some() {
        return Ok(());
    }
    let mut command = Cli::command();
    let clap_error = command
        .error(
            ErrorKind::MissingRequiredArgument,
            KEYRING_IDENTITY_REQUIRED_MESSAGE,
        )
        .with_cmd(&command);
    Err(clap_error.into())
}

pub(crate) fn build_cli_command_context(
    global: &cli::GlobalOpts,
) -> Result<(AgentFactory, Option<PathBuf>)> {
    if global.ii {
        let identity_path = resolve_identity_path(global)?;
        let delegated = identity_store::load_delegated_identity(&identity_path)?;
        Ok((
            AgentFactory::new_with_identity(global.ic, delegated),
            Some(identity_path),
        ))
    } else {
        let identity = resolve_required_identity(global)?;
        Ok((build_keyring_agent_factory(global.ic, &identity), None))
    }
}

pub(crate) fn build_keyring_agent_factory(use_mainnet: bool, identity: &str) -> AgentFactory {
    AgentFactory::new(use_mainnet, identity.to_string())
}

pub(crate) fn resolve_tui_identity(global: &cli::GlobalOpts) -> Result<String> {
    global
        .identity
        .clone()
        .ok_or_else(|| anyhow!(TUI_IDENTITY_REQUIRED_MESSAGE))
}

fn resolve_required_identity(global: &cli::GlobalOpts) -> Result<String> {
    global
        .identity
        .clone()
        .ok_or_else(|| anyhow!(KEYRING_IDENTITY_REQUIRED_MESSAGE))
}

fn resolve_identity_path(global: &cli::GlobalOpts) -> Result<PathBuf> {
    match global.identity_path.clone() {
        Some(path) => Ok(path),
        None => identity_store::default_identity_path(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn global_opts(
        identity: Option<&str>,
        ii: bool,
        identity_path: Option<PathBuf>,
    ) -> cli::GlobalOpts {
        cli::GlobalOpts {
            verbose: 0,
            ic: false,
            identity: identity.map(ToOwned::to_owned),
            ii,
            identity_path,
        }
    }

    #[test]
    fn resolve_tui_identity_returns_identity_when_present() {
        let global = global_opts(Some("alice"), false, None);

        let identity = resolve_tui_identity(&global).unwrap();

        assert_eq!(identity, "alice");
    }

    #[test]
    fn resolve_tui_identity_requires_identity() {
        let global = global_opts(None, false, None);

        let error = resolve_tui_identity(&global).unwrap_err();

        assert_eq!(error.to_string(), TUI_IDENTITY_REQUIRED_MESSAGE);
    }

    #[test]
    fn resolve_identity_path_uses_default_location_when_not_provided() {
        let global = global_opts(None, true, None);

        let path = resolve_identity_path(&global).unwrap();

        assert!(path.ends_with(PathBuf::from(".config/kinic/identity.json")));
    }

    #[test]
    fn validate_tui_cli_args_accepts_identity_for_tui_command() {
        let cli = Cli::try_parse_from(["kinic-cli", "--identity", "alice", "tui"])
            .expect("cli parsing should succeed");

        validate_tui_cli_args(&cli).expect("validation should accept tui with identity");
    }

    #[test]
    fn validate_tui_cli_args_rejects_ii_for_tui() {
        let cli =
            Cli::try_parse_from(["kinic-cli", "--ii", "tui"]).expect("cli parsing should succeed");

        let error = validate_tui_cli_args(&cli).unwrap_err();
        let clap_error = error.downcast_ref::<clap::Error>().unwrap();

        assert_eq!(clap_error.kind(), ErrorKind::ArgumentConflict);
        assert!(clap_error.to_string().contains(TUI_II_UNSUPPORTED_MESSAGE));
    }

    #[test]
    fn cli_rejects_empty_identity_for_tui_command() {
        let error = Cli::try_parse_from(["kinic-cli", "--identity", "", "tui"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn cli_rejects_whitespace_only_identity_for_tui_command() {
        let error = Cli::try_parse_from(["kinic-cli", "--identity", "   ", "tui"]).unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::ValueValidation);
    }

    #[test]
    fn validate_keyring_identity_rejects_list_without_identity() {
        let cli = Cli::try_parse_from(["kinic-cli", "list"]).expect("cli parse");

        let error = validate_keyring_identity(&cli).unwrap_err();
        let clap_error = error.downcast_ref::<clap::Error>().unwrap();

        assert_eq!(clap_error.kind(), ErrorKind::MissingRequiredArgument);
        assert!(
            clap_error
                .to_string()
                .contains(KEYRING_IDENTITY_REQUIRED_MESSAGE)
        );
    }

    #[test]
    fn validate_keyring_identity_accepts_list_with_identity() {
        let cli =
            Cli::try_parse_from(["kinic-cli", "--identity", "alice", "list"]).expect("cli parse");

        validate_keyring_identity(&cli).expect("identity present");
    }

    #[test]
    fn validate_keyring_identity_accepts_list_with_ii() {
        let cli = Cli::try_parse_from(["kinic-cli", "--ii", "list"]).expect("cli parse");

        validate_keyring_identity(&cli).expect("ii avoids keyring identity");
    }

    #[test]
    fn validate_keyring_identity_skips_tui_command() {
        let cli = Cli::try_parse_from(["kinic-cli", "--identity", "alice", "tui"]).expect("cli");

        validate_keyring_identity(&cli).expect("tui handled by validate_tui_cli_args");
    }

    #[test]
    fn validate_keyring_identity_skips_prefs_command_without_identity() {
        let cli = Cli::try_parse_from(["kinic-cli", "prefs", "show"]).expect("cli");

        validate_keyring_identity(&cli).expect("prefs should not require identity");
    }

    #[test]
    fn validate_keyring_identity_skips_capabilities_without_identity() {
        let cli = Cli::try_parse_from(["kinic-cli", "capabilities"]).expect("cli");

        validate_keyring_identity(&cli).expect("capabilities should not require identity");
    }

    #[test]
    fn cli_parses_prefs_show_without_identity() {
        let cli = Cli::try_parse_from(["kinic-cli", "prefs", "show"]).expect("cli");

        assert!(matches!(
            cli.command,
            cli::Command::Prefs(cli::PrefsArgs {
                command: cli::PrefsCommand::Show
            })
        ));
    }

    #[test]
    fn log_level_defaults_to_warn() {
        assert_eq!(log_level_for_verbose(0), LevelFilter::WARN);
    }

    #[test]
    fn log_level_maps_verbose_flags_progressively() {
        assert_eq!(log_level_for_verbose(1), LevelFilter::INFO);
        assert_eq!(log_level_for_verbose(2), LevelFilter::DEBUG);
        assert_eq!(log_level_for_verbose(3), LevelFilter::TRACE);
        assert_eq!(log_level_for_verbose(9), LevelFilter::TRACE);
    }
}

#[cfg(feature = "python-bindings")]
#[pymodule]
fn _lib(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(greet, m)?)?;
    m.add_function(wrap_pyfunction!(create_memory, m)?)?;
    m.add_function(wrap_pyfunction!(list_memories, m)?)?;
    m.add_function(wrap_pyfunction!(insert_memory, m)?)?;
    m.add_function(wrap_pyfunction!(insert_memory_raw, m)?)?;
    m.add_function(wrap_pyfunction!(insert_memory_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(search_memories, m)?)?;
    m.add_function(wrap_pyfunction!(search_memories_raw, m)?)?;
    m.add_function(wrap_pyfunction!(tagged_embeddings, m)?)?;
    m.add_function(wrap_pyfunction!(ask_ai, m)?)?;
    m.add_function(wrap_pyfunction!(get_balance, m)?)?;
    m.add_function(wrap_pyfunction!(update_instance, m)?)?;
    m.add_function(wrap_pyfunction!(reset_memory, m)?)?;
    m.add_function(wrap_pyfunction!(add_user, m)?)?;
    Ok(())
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
fn greet() -> PyResult<String> {
    Ok("hello!".to_string())
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, name, description, ic=None))]
fn create_memory(
    identity: &str,
    name: &str,
    description: &str,
    ic: Option<bool>,
) -> PyResult<String> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::create_memory(
        ic,
        identity.to_string(),
        name.to_string(),
        description.to_string(),
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, ic=None))]
fn list_memories(identity: &str, ic: Option<bool>) -> PyResult<Vec<String>> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::list_memories(ic, identity.to_string()))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, tag, text=None, file_path=None, ic=None))]
fn insert_memory(
    identity: &str,
    memory_id: &str,
    tag: &str,
    text: Option<&str>,
    file_path: Option<&str>,
    ic: Option<bool>,
) -> PyResult<usize> {
    if text.is_none() && file_path.is_none() {
        return Err(PyValueError::new_err(
            "either `text` or `file_path` must be provided",
        ));
    }

    let ic = ic.unwrap_or(false);
    let path = file_path.map(PathBuf::from);
    block_on_py(python::insert_memory(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        tag.to_string(),
        text.map(|t| t.to_string()),
        path,
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, tag, text, embedding, ic=None))]
fn insert_memory_raw(
    identity: &str,
    memory_id: &str,
    tag: &str,
    text: &str,
    embedding: Vec<f32>,
    ic: Option<bool>,
) -> PyResult<usize> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::insert_memory_raw(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        tag.to_string(),
        text.to_string(),
        embedding,
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, tag, file_path, ic=None))]
fn insert_memory_pdf(
    identity: &str,
    memory_id: &str,
    tag: &str,
    file_path: &str,
    ic: Option<bool>,
) -> PyResult<usize> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::insert_memory_pdf(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        tag.to_string(),
        PathBuf::from(file_path),
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, query, ic=None))]
fn search_memories(
    identity: &str,
    memory_id: &str,
    query: &str,
    ic: Option<bool>,
) -> PyResult<Vec<(f32, String)>> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::search_memories(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        query.to_string(),
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, embedding, ic=None))]
fn search_memories_raw(
    identity: &str,
    memory_id: &str,
    embedding: Vec<f32>,
    ic: Option<bool>,
) -> PyResult<Vec<(f32, String)>> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::search_memories_raw(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        embedding,
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, tag, ic=None))]
fn tagged_embeddings(
    identity: &str,
    memory_id: &str,
    tag: &str,
    ic: Option<bool>,
) -> PyResult<Vec<Vec<f32>>> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::tagged_embeddings(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        tag.to_string(),
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, query, top_k=None, language=None, ic=None))]
fn ask_ai(
    identity: &str,
    memory_id: &str,
    query: &str,
    top_k: Option<usize>,
    language: Option<&str>,
    ic: Option<bool>,
) -> PyResult<(String, String)> {
    let ic = ic.unwrap_or(false);
    let language = language.map(|s| s.to_string());
    let result = block_on_py(python::ask_ai(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        query.to_string(),
        top_k,
        language,
    ))?;
    Ok((result.prompt, result.response))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, ic=None))]
fn get_balance(identity: &str, ic: Option<bool>) -> PyResult<(u128, f64)> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::balance(ic, identity.to_string()))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, ic=None))]
fn update_instance(identity: &str, memory_id: &str, ic: Option<bool>) -> PyResult<()> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::update_instance(
        ic,
        identity.to_string(),
        memory_id.to_string(),
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, dim, ic=None))]
fn reset_memory(identity: &str, memory_id: &str, dim: usize, ic: Option<bool>) -> PyResult<()> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::reset_memory(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        dim,
    ))
}

#[cfg(feature = "python-bindings")]
#[pyfunction]
#[pyo3(signature = (identity, memory_id, user_id, role, ic=None))]
fn add_user(
    identity: &str,
    memory_id: &str,
    user_id: &str,
    role: &str,
    ic: Option<bool>,
) -> PyResult<()> {
    let ic = ic.unwrap_or(false);
    block_on_py(python::add_user(
        ic,
        identity.to_string(),
        memory_id.to_string(),
        user_id.to_string(),
        role.to_string(),
    ))
}

#[cfg(feature = "python-bindings")]
fn block_on_py<F, T>(future: F) -> PyResult<T>
where
    F: std::future::Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    Runtime::new()
        .map_err(|e| PyRuntimeError::new_err(format!("failed to start tokio runtime: {e}")))?
        .block_on(future)
        .map_err(anyhow_to_pyerr)
}

#[cfg(feature = "python-bindings")]
fn anyhow_to_pyerr(err: anyhow::Error) -> PyErr {
    PyRuntimeError::new_err(format!("{err:?}"))
}
