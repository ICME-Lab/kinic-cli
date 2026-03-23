pub mod agent;
#[path = "cli_defs.rs"]
pub mod cli;
pub(crate) mod clients;
mod commands;
mod embedding;
pub(crate) mod identity_store;
mod ledger;
#[cfg(feature = "python-bindings")]
mod python;
pub mod tui;

use anyhow::Result;
use clap::Parser;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt;

use crate::{
    agent::AgentFactory,
    cli::Cli,
    commands::{CommandContext, run_command},
};

#[cfg(feature = "python-bindings")]
use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*,
    types::PyModule,
    wrap_pyfunction,
};
#[cfg(feature = "python-bindings")]
use std::path::PathBuf;
#[cfg(feature = "python-bindings")]
use tokio::runtime::Runtime;

pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    let max = match cli.global.verbose {
        0 => LevelFilter::INFO,
        1 => LevelFilter::DEBUG,
        _ => LevelFilter::TRACE,
    };

    fmt().with_max_level(max).without_time().try_init().ok();

    if matches!(&cli.command, cli::Command::Tui(_)) {
        return tui::run(&cli.global);
    }

    if cli.global.ii
        && matches!(
            &cli.command,
            cli::Command::Create(_) | cli::Command::Balance(_)
        )
    {
        if !cfg!(feature = "experimental") {
            anyhow::bail!(
                "For security reasons, using a locally hosted origin Internet Identity is not recommended for commands involving asset transfers."
            );
        }
    }

    if !cli.global.ii
        && cli.global.identity.is_none()
        && !matches!(&cli.command, cli::Command::Login(_) | cli::Command::Tui(_))
    {
        anyhow::bail!("--identity is required unless --ii is set");
    }

    let needs_identity_path = matches!(&cli.command, cli::Command::Login(_)) || cli.global.ii;
    let identity_path = if needs_identity_path {
        Some(match cli.global.identity_path.clone() {
            Some(path) => path,
            None => identity_store::default_identity_path()?,
        })
    } else {
        None
    };

    let agent_factory = if matches!(&cli.command, cli::Command::Login(_)) {
        AgentFactory::new(cli.global.ic, String::new())
    } else if cli.global.ii {
        let path = identity_path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("Identity path is missing"))?;
        let delegated = identity_store::load_delegated_identity(&path)?;
        AgentFactory::new_with_identity(cli.global.ic, delegated)
    } else {
        let identity_suffix = cli
            .global
            .identity
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--identity is required unless --ii is set"))?;
        AgentFactory::new(cli.global.ic, identity_suffix)
    };

    let context = CommandContext {
        agent_factory,
        identity_path,
    };

    run_command(cli.command, context).await
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
fn reset_memory(
    identity: &str,
    memory_id: &str,
    dim: usize,
    ic: Option<bool>,
) -> PyResult<()> {
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
