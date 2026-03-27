use serde::{Serialize, de::DeserializeOwned};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

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

pub fn load_yaml_or_default<T>(app_namespace: &str, file_name: &str) -> Result<T, SettingsError>
where
    T: Default + DeserializeOwned,
{
    let path = config_file_path(app_namespace, file_name)?;
    load_yaml_or_default_at_path(path.as_path())
}

pub fn save_yaml<T>(app_namespace: &str, file_name: &str, value: &T) -> Result<(), SettingsError>
where
    T: Serialize,
{
    let path = config_file_path(app_namespace, file_name)?;
    save_yaml_at_path(path.as_path(), value)
}

fn load_yaml_or_default_at_path<T>(path: &Path) -> Result<T, SettingsError>
where
    T: Default + DeserializeOwned,
{
    if !path.exists() {
        return Ok(T::default());
    }

    let content = std::fs::read_to_string(path)?;
    Ok(serde_yaml::from_str(&content)?)
}

fn save_yaml_at_path<T>(path: &Path, value: &T) -> Result<(), SettingsError>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_yaml::to_string(value)?;
    atomic_write(path, content.as_bytes())?;
    Ok(())
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), SettingsError> {
    let parent = path.parent().ok_or(SettingsError::NoConfigDir)?;
    let temp_path = temp_file_path(parent, path.file_name().and_then(|name| name.to_str()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)?;

    let write_result = (|| -> std::io::Result<()> {
        file.write_all(content)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    })();

    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(SettingsError::Io(error));
    }

    if let Err(error) = replace_file(temp_path.as_path(), path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(SettingsError::Io(error));
    }

    Ok(())
}

fn temp_file_path(parent: &Path, file_name: Option<&str>) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let suffix = COUNTER.fetch_add(1, Ordering::Relaxed);
    let base = file_name.unwrap_or("settings");
    parent.join(format!(".{base}.tmp-{}-{suffix}", std::process::id()))
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    type Bool = i32;
    type Dword = u32;
    type Lpcwstr = *const u16;

    const MOVEFILE_REPLACE_EXISTING: Dword = 0x1;
    const MOVEFILE_WRITE_THROUGH: Dword = 0x8;

    unsafe extern "system" {
        fn MoveFileExW(
            lp_existing_file_name: Lpcwstr,
            lp_new_file_name: Lpcwstr,
            dw_flags: Dword,
        ) -> Bool;
    }

    fn to_wide(path: &Path) -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    let from_wide = to_wide(from);
    let to_wide = to_wide(to);
    let result = unsafe {
        MoveFileExW(
            from_wide.as_ptr(),
            to_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };

    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
#[path = "settings_tests.rs"]
mod tests;
