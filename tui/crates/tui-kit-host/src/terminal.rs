//! Terminal enter/leave helpers for host loops and GUI picker suspension.
//!
//! This module keeps terminal restoration explicit so GUI dialogs cannot leave
//! the host loop in a half-restored state.

use std::{fmt, io, path::PathBuf};

use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tui_kit_runtime::InsertMode;

#[cfg(feature = "rfd-file-picker")]
use tui_kit_runtime::FILE_MODE_ALLOWED_EXTENSIONS;

pub type HostTerminal = Terminal<CrosstermBackend<io::Stdout>>;
pub type FilePickerFn = fn(InsertMode) -> Result<Option<PathBuf>, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickFilePathError {
    Picker(String),
    TerminalState(String),
}

impl fmt::Display for PickFilePathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Picker(message) | Self::TerminalState(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for PickFilePathError {}

fn enter_terminal(terminal: &mut HostTerminal) -> io::Result<()> {
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.hide_cursor()?;
    Ok(())
}

fn leave_terminal(terminal: &mut HostTerminal) -> io::Result<()> {
    terminal.show_cursor()?;
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    Ok(())
}

struct SuspendedTerminal<'a> {
    terminal: &'a mut HostTerminal,
    suspended: bool,
}

impl<'a> SuspendedTerminal<'a> {
    fn new(terminal: &'a mut HostTerminal) -> Result<Self, PickFilePathError> {
        leave_terminal(terminal)
            .map_err(|error| PickFilePathError::TerminalState(error.to_string()))?;
        Ok(Self {
            terminal,
            suspended: true,
        })
    }

    fn restore(&mut self) -> Result<(), PickFilePathError> {
        enter_terminal(self.terminal)
            .map_err(|error| PickFilePathError::TerminalState(error.to_string()))?;
        self.terminal
            .clear()
            .map_err(|error| PickFilePathError::TerminalState(error.to_string()))?;
        self.suspended = false;
        Ok(())
    }
}

impl Drop for SuspendedTerminal<'_> {
    fn drop(&mut self) {
        if self.suspended {
            let _ = enter_terminal(self.terminal);
            let _ = self.terminal.clear();
        }
    }
}

pub fn with_terminal<R, F>(run: F) -> Result<R, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut HostTerminal) -> Result<R, Box<dyn std::error::Error>>,
{
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    enter_terminal(&mut terminal)?;

    let result = run(&mut terminal);
    let cleanup = leave_terminal(&mut terminal);

    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(Box::new(error)),
        (Err(run_error), Err(_cleanup_error)) => Err(run_error),
    }
}

pub fn pick_file_path(
    terminal: &mut HostTerminal,
    picker: FilePickerFn,
    insert_mode: InsertMode,
) -> Result<Option<PathBuf>, PickFilePathError> {
    let mut suspended = SuspendedTerminal::new(terminal)?;
    match picker(insert_mode) {
        Ok(selection) => {
            suspended.restore()?;
            Ok(selection)
        }
        Err(error) => match suspended.restore() {
            Ok(()) => Err(PickFilePathError::Picker(error)),
            Err(restore_error) => Err(restore_error),
        },
    }
}

pub fn default_file_picker() -> Option<FilePickerFn> {
    #[cfg(feature = "rfd-file-picker")]
    {
        Some(rfd_file_picker)
    }

    #[cfg(not(feature = "rfd-file-picker"))]
    {
        None
    }
}

#[cfg(feature = "rfd-file-picker")]
pub fn rfd_file_picker(insert_mode: InsertMode) -> Result<Option<PathBuf>, String> {
    use rfd::FileDialog;

    let mut dialog = FileDialog::new().set_title("Select file");
    if matches!(insert_mode, InsertMode::File) {
        dialog = dialog.add_filter("Supported files", FILE_MODE_ALLOWED_EXTENSIONS);
    }
    Ok(dialog.pick_file())
}

#[cfg(test)]
fn pick_file_path_with_ops<FLeave, FPick, FEnter, FClear>(
    mut leave: FLeave,
    mut pick: FPick,
    mut enter: FEnter,
    mut clear: FClear,
) -> Result<Option<PathBuf>, PickFilePathError>
where
    FLeave: FnMut() -> Result<(), String>,
    FPick: FnMut() -> Result<Option<PathBuf>, String>,
    FEnter: FnMut() -> Result<(), String>,
    FClear: FnMut() -> Result<(), String>,
{
    leave().map_err(PickFilePathError::TerminalState)?;
    match pick() {
        Ok(selection) => {
            enter().map_err(PickFilePathError::TerminalState)?;
            clear().map_err(PickFilePathError::TerminalState)?;
            Ok(selection)
        }
        Err(error) => {
            match enter().map_err(PickFilePathError::TerminalState) {
                Ok(()) => match clear().map_err(PickFilePathError::TerminalState) {
                    Ok(()) => Err(PickFilePathError::Picker(error)),
                    Err(restore_error) => Err(restore_error),
                },
                Err(restore_error) => Err(restore_error),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    #[test]
    fn pick_file_path_sequence_restores_terminal_after_selection() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let shared = events.clone();
        let result = pick_file_path_with_ops(
            || {
                shared.borrow_mut().push("leave");
                Ok(())
            },
            || {
                shared.borrow_mut().push("pick");
                Ok(Some(PathBuf::from("/tmp/doc.md")))
            },
            || {
                shared.borrow_mut().push("enter");
                Ok(())
            },
            || {
                shared.borrow_mut().push("clear");
                Ok(())
            },
        )
        .expect("picker should succeed");

        assert_eq!(result, Some(PathBuf::from("/tmp/doc.md")));
        assert_eq!(events.borrow().as_slice(), ["leave", "pick", "enter", "clear"]);
    }

    #[test]
    fn pick_file_path_treats_enter_failure_as_fatal_restore_error() {
        let error = pick_file_path_with_ops(
            || Ok(()),
            || Ok(None),
            || Err("enter failed".to_string()),
            || Ok(()),
        )
        .expect_err("enter failure should be fatal");

        assert_eq!(
            error,
            PickFilePathError::TerminalState("enter failed".to_string())
        );
    }

    #[test]
    fn pick_file_path_treats_clear_failure_as_fatal_restore_error() {
        let error = pick_file_path_with_ops(
            || Ok(()),
            || Ok(None),
            || Ok(()),
            || Err("clear failed".to_string()),
        )
        .expect_err("clear failure should be fatal");

        assert_eq!(
            error,
            PickFilePathError::TerminalState("clear failed".to_string())
        );
    }

    #[test]
    fn pick_file_path_treats_leave_failure_as_fatal_terminal_error() {
        let error = pick_file_path_with_ops(
            || Err("leave failed".to_string()),
            || Ok(None),
            || Ok(()),
            || Ok(()),
        )
        .expect_err("leave failure should be fatal");

        assert_eq!(
            error,
            PickFilePathError::TerminalState("leave failed".to_string())
        );
    }

    #[test]
    fn pick_file_path_returns_picker_error_only_after_successful_restore() {
        let error = pick_file_path_with_ops(
            || Ok(()),
            || Err("dialog failed".to_string()),
            || Ok(()),
            || Ok(()),
        )
        .expect_err("picker failure should return picker error after restore");

        assert_eq!(error, PickFilePathError::Picker("dialog failed".to_string()));
    }

    #[test]
    fn pick_file_path_prioritizes_restore_failure_after_picker_error() {
        let error = pick_file_path_with_ops(
            || Ok(()),
            || Err("dialog failed".to_string()),
            || Err("enter failed".to_string()),
            || Ok(()),
        )
        .expect_err("restore failure should be fatal");

        assert_eq!(
            error,
            PickFilePathError::TerminalState("enter failed".to_string())
        );
    }

    #[cfg(not(feature = "rfd-file-picker"))]
    #[test]
    fn host_builds_without_rfd_picker_feature() {
        assert!(!cfg!(feature = "rfd-file-picker"));
    }
}
