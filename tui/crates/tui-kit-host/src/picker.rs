//! Picker backends for host loops.
//!
//! Keeps file selection outside the TUI process so the host only needs to
//! suspend the terminal, launch the chooser, and restore the terminal state.

use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

use tempfile::NamedTempFile;
use tracing::{debug, info};
use tui_kit_runtime::InsertMode;

pub trait PickerBackend {
    fn pick_file(&mut self, cwd: &Path, insert_mode: InsertMode)
    -> Result<Option<PathBuf>, String>;
}

pub fn default_picker_backend() -> Option<Box<dyn PickerBackend>> {
    Some(Box::new(ExternalChooser::for_yazi()))
}

pub struct ExternalChooser {
    program: String,
    args_template: Vec<String>,
}

impl ExternalChooser {
    pub fn for_yazi() -> Self {
        Self {
            program: "yazi".to_string(),
            args_template: vec![
                "{cwd}".to_string(),
                "--chooser-file".to_string(),
                "{chooser_file}".to_string(),
            ],
        }
    }

    #[cfg(test)]
    fn new_for_tests(program: &str, args_template: Vec<String>) -> Self {
        Self {
            program: program.to_string(),
            args_template,
        }
    }

    fn render_args(&self, cwd: &Path, chooser_file: &Path) -> Vec<String> {
        let cwd = cwd.display().to_string();
        let chooser_file = chooser_file.display().to_string();
        self.args_template
            .iter()
            .map(|arg| {
                arg.replace("{cwd}", &cwd)
                    .replace("{chooser_file}", &chooser_file)
            })
            .collect()
    }

    fn chooser_output_path() -> io::Result<tempfile::TempPath> {
        Ok(NamedTempFile::new()?.into_temp_path())
    }

    fn missing_program_message(&self) -> String {
        format!(
            "File chooser requires '{}'. Install it first (for example on macOS: `brew install {}`), or enter the FilePath manually.",
            self.program, self.program
        )
    }

    fn chooser_exit_message(&self, status: std::process::ExitStatus) -> String {
        match status.code() {
            Some(code) => format!(
                "External chooser '{}' exited with status {code} before returning a selection.",
                self.program
            ),
            None => format!(
                "External chooser '{}' terminated before returning a selection.",
                self.program
            ),
        }
    }
}

impl PickerBackend for ExternalChooser {
    fn pick_file(
        &mut self,
        cwd: &Path,
        insert_mode: InsertMode,
    ) -> Result<Option<PathBuf>, String> {
        let chooser_file = Self::chooser_output_path().map_err(|error| error.to_string())?;
        let chooser_path: &Path = chooser_file.as_ref();
        let args = self.render_args(cwd, chooser_path);
        info!(
            program = %self.program,
            cwd = %cwd.display(),
            insert_mode = ?insert_mode,
            "launching external chooser"
        );

        let status = Command::new(&self.program)
            .args(&args)
            .current_dir(cwd)
            .status()
            .map_err(|error| {
                if error.kind() == io::ErrorKind::NotFound {
                    self.missing_program_message()
                } else {
                    format!(
                        "Failed to launch external chooser '{}': {error}",
                        self.program
                    )
                }
            })?;

        let chooser_output = std::fs::read_to_string(chooser_path).map_err(|error| {
            format!(
                "Failed to read chooser output '{}': {error}",
                chooser_file.display()
            )
        })?;
        let selected_path = chooser_output
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .map(PathBuf::from);

        debug!(
            program = %self.program,
            status = ?status.code(),
            has_selection = selected_path.is_some(),
            "external chooser finished"
        );

        let Some(selected_path) = selected_path else {
            if !status.success() {
                return Err(self.chooser_exit_message(status));
            }
            return Ok(None);
        };

        let resolved = if selected_path.is_absolute() {
            selected_path
        } else {
            cwd.join(selected_path)
        };
        info!(path = %resolved.display(), "external chooser selected path");
        Ok(Some(resolved))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};

    fn write_script(dir: &Path, name: &str, body: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, body).expect("script should be written");
        let mut permissions = fs::metadata(&path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("permissions should be set");
        path
    }

    #[test]
    fn external_chooser_resolves_relative_paths_from_cwd() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let script = write_script(
            temp_dir.path(),
            "chooser.sh",
            "#!/bin/sh\nprintf 'docs/file.md\\n' > \"$1\"\n",
        );
        let mut chooser = ExternalChooser::new_for_tests(
            script.to_str().expect("script path"),
            vec!["{chooser_file}".to_string()],
        );

        let result = chooser
            .pick_file(temp_dir.path(), InsertMode::File)
            .expect("chooser should succeed");

        assert_eq!(result, Some(temp_dir.path().join("docs/file.md")));
    }

    #[test]
    fn external_chooser_treats_empty_output_as_cancel() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let script = write_script(temp_dir.path(), "chooser.sh", "#!/bin/sh\n: > \"$1\"\n");
        let mut chooser = ExternalChooser::new_for_tests(
            script.to_str().expect("script path"),
            vec!["{chooser_file}".to_string()],
        );

        let result = chooser
            .pick_file(temp_dir.path(), InsertMode::File)
            .expect("chooser should succeed");

        assert_eq!(result, None);
    }

    #[test]
    fn external_chooser_returns_error_on_nonzero_exit_without_selection() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let script = write_script(
            temp_dir.path(),
            "chooser.sh",
            "#!/bin/sh\n: > \"$1\"\nexit 2\n",
        );
        let mut chooser = ExternalChooser::new_for_tests(
            script.to_str().expect("script path"),
            vec!["{chooser_file}".to_string()],
        );

        let error = chooser
            .pick_file(temp_dir.path(), InsertMode::File)
            .expect_err("nonzero exit without selection should error");

        assert_eq!(
            error,
            format!(
                "External chooser '{}' exited with status 2 before returning a selection.",
                script.to_str().expect("script path")
            )
        );
    }

    #[test]
    fn external_chooser_returns_error_when_program_is_missing() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let mut chooser = ExternalChooser::new_for_tests("/missing-chooser", Vec::new());

        let error = chooser
            .pick_file(temp_dir.path(), InsertMode::File)
            .expect_err("missing program should error");

        assert_eq!(
            error,
            "File chooser requires '/missing-chooser'. Install it first (for example on macOS: `brew install /missing-chooser`), or enter the FilePath manually."
        );
    }

    #[test]
    fn external_chooser_prefers_chooser_file_even_on_nonzero_exit() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let script = write_script(
            temp_dir.path(),
            "chooser.sh",
            "#!/bin/sh\nprintf '/tmp/result.pdf\\n' > \"$1\"\nexit 1\n",
        );
        let mut chooser = ExternalChooser::new_for_tests(
            script.to_str().expect("script path"),
            vec!["{chooser_file}".to_string()],
        );

        let result = chooser
            .pick_file(temp_dir.path(), InsertMode::File)
            .expect("chooser should still return selection");

        assert_eq!(result, Some(PathBuf::from("/tmp/result.pdf")));
    }
}
