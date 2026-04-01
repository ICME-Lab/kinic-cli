use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use rfd::FileDialog;
use tui_kit_runtime::InsertMode;

pub type HostTerminal = Terminal<CrosstermBackend<io::Stdout>>;

fn enter_terminal(terminal: &mut HostTerminal) -> io::Result<()> {
    enable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        EnterAlternateScreen,
        EnableMouseCapture
    )?;
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

pub fn with_terminal<R, F>(run: F) -> Result<R, Box<dyn std::error::Error>>
where
    F: FnOnce(&mut HostTerminal) -> Result<R, Box<dyn std::error::Error>>,
{
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    enter_terminal(&mut terminal)?;

    let result = run(&mut terminal);

    leave_terminal(&mut terminal)?;

    result
}

pub fn pick_file_path(
    terminal: &mut HostTerminal,
    _insert_mode: InsertMode,
) -> Result<Option<std::path::PathBuf>, String> {
    leave_terminal(terminal).map_err(|error| error.to_string())?;

    let selection = FileDialog::new().set_title("Select file").pick_file();

    enter_terminal(terminal).map_err(|error| error.to_string())?;
    terminal.clear().map_err(|error| error.to_string())?;
    Ok(selection)
}
