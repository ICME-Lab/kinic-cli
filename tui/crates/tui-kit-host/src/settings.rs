use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;

#[derive(Debug)]
pub enum SettingsError {
    NoConfigDir,
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoConfigDir => write!(f, "No config directory found"),
            Self::Io(e) => write!(f, "I/O error: {e}"),
            Self::Yaml(e) => write!(f, "YAML error: {e}"),
        }
    }
}

impl std::error::Error for SettingsError {}

impl From<std::io::Error> for SettingsError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_yaml::Error> for SettingsError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Yaml(value)
    }
}

pub fn config_file_path(app_namespace: &str, file_name: &str) -> Result<PathBuf, SettingsError> {
    let base = dirs::config_dir().ok_or(SettingsError::NoConfigDir)?;
    Ok(base.join(app_namespace).join(file_name))
}

pub fn load_yaml_or_default<T>(
    app_namespace: &str,
    file_name: &str,
) -> Result<T, SettingsError>
where
    T: Default + DeserializeOwned,
{
    let path = config_file_path(app_namespace, file_name)?;
    if !path.exists() {
        return Ok(T::default());
    }

    let content = std::fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&content)?)
}

pub fn save_yaml<T>(app_namespace: &str, file_name: &str, value: &T) -> Result<(), SettingsError>
where
    T: Serialize,
{
    let path = config_file_path(app_namespace, file_name)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_yaml::to_string(value)?;
    std::fs::write(path, content)?;
    Ok(())
}

