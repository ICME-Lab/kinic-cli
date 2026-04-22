//! Terminal enter/leave helpers for host loops and external chooser suspension.
//!
//! This module keeps terminal restoration explicit so external choosers cannot
//! leave the host loop in a half-restored state.

use std::{
    fmt, io,
    path::{Path, PathBuf},
};

use crossterm::{
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
        supports_keyboard_enhancement,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tui_kit_runtime::InsertMode;

use crate::picker::PickerBackend;

pub type HostTerminal = Terminal<CrosstermBackend<io::Stdout>>;

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

fn enter_terminal(terminal: &mut HostTerminal) -> io::Result<bool> {
    enable_raw_mode()?;
    let keyboard_enhancement_enabled = supports_keyboard_enhancement().unwrap_or(false);
    if keyboard_enhancement_enabled {
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableBracketedPaste,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
            )
        )?;
    } else {
        execute!(
            terminal.backend_mut(),
            EnterAlternateScreen,
            EnableBracketedPaste
        )?;
    }
    terminal.hide_cursor()?;
    Ok(keyboard_enhancement_enabled)
}

fn leave_terminal(
    terminal: &mut HostTerminal,
    keyboard_enhancement_enabled: bool,
) -> io::Result<()> {
    terminal.show_cursor()?;
    disable_raw_mode()?;
    if keyboard_enhancement_enabled {
        execute!(
            terminal.backend_mut(),
            PopKeyboardEnhancementFlags,
            DisableBracketedPaste,
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
    } else {
        execute!(
            terminal.backend_mut(),
            DisableBracketedPaste,
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
    }
    Ok(())
}

struct SuspendedTerminal<'a> {
    terminal: &'a mut HostTerminal,
    keyboard_enhancement_enabled: bool,
    suspended: bool,
}

impl<'a> SuspendedTerminal<'a> {
    fn new(
        terminal: &'a mut HostTerminal,
        keyboard_enhancement_enabled: bool,
    ) -> Result<Self, PickFilePathError> {
        leave_terminal(terminal, keyboard_enhancement_enabled)
            .map_err(|error| PickFilePathError::TerminalState(error.to_string()))?;
        Ok(Self {
            terminal,
            keyboard_enhancement_enabled,
            suspended: true,
        })
    }

    fn restore(&mut self) -> Result<(), PickFilePathError> {
        self.keyboard_enhancement_enabled = enter_terminal(self.terminal)
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
            if let Ok(enabled) = enter_terminal(self.terminal) {
                self.keyboard_enhancement_enabled = enabled;
            }
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
    let keyboard_enhancement_enabled = enter_terminal(&mut terminal)?;

    let result = run(&mut terminal);
    let cleanup = leave_terminal(&mut terminal, keyboard_enhancement_enabled);

    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(Box::new(error)),
        (Err(run_error), Err(_cleanup_error)) => Err(run_error),
    }
}

pub fn pick_file_path(
    terminal: &mut HostTerminal,
    picker: &mut dyn PickerBackend,
    cwd: &Path,
    insert_mode: InsertMode,
) -> Result<Option<PathBuf>, PickFilePathError> {
    let keyboard_enhancement_enabled = supports_keyboard_enhancement().unwrap_or(false);
    let mut suspended = SuspendedTerminal::new(terminal, keyboard_enhancement_enabled)?;
    match picker.pick_file(cwd, insert_mode) {
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

#[cfg(test)]
fn pick_file_path_with_ops<FLeave, FPick, FEnter, FClear>(
    mut leave: FLeave,
    mut pick: FPick,
    mut enter: FEnter,
    mut clear: FClear,
) -> Result<Option<PathBuf>, PickFilePathError>
where
    FLeave: FnMut(bool) -> Result<(), String>,
    FPick: FnMut() -> Result<Option<PathBuf>, String>,
    FEnter: FnMut() -> Result<bool, String>,
    FClear: FnMut() -> Result<(), String>,
{
    leave(false).map_err(PickFilePathError::TerminalState)?;
    match pick() {
        Ok(selection) => {
            let _keyboard_enhancement_enabled =
                enter().map_err(PickFilePathError::TerminalState)?;
            clear().map_err(PickFilePathError::TerminalState)?;
            Ok(selection)
        }
        Err(error) => match enter().map_err(PickFilePathError::TerminalState) {
            Ok(_keyboard_enhancement_enabled) => {
                match clear().map_err(PickFilePathError::TerminalState) {
                    Ok(()) => Err(PickFilePathError::Picker(error)),
                    Err(restore_error) => Err(restore_error),
                }
            }
            Err(restore_error) => Err(restore_error),
        },
    }
}

#[cfg(test)]
#[path = "terminal_tests.rs"]
mod tests;
