pub mod agent;
#[path = "cli_defs.rs"]
pub mod cli;
pub(crate) mod clients;
mod commands;
mod embedding;
#[cfg(feature = "python-bindings")]
mod python;

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

    let context = CommandContext {
        agent_factory: AgentFactory::new(cli.global.ic, cli.global.identity.clone()),
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
    m.add_function(wrap_pyfunction!(search_memories, m)?)?;
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
