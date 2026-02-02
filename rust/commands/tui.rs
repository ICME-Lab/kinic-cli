use anyhow::Result;
use std::io::{self, Write};

use indicatif::{ProgressBar, ProgressStyle};
use std::thread::sleep;
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use dialoguer::Select;

use crate::commands::CommandContext;

pub async fn handle(_ctx: &CommandContext) -> Result<()> {
    run_menu()
}

fn run_menu() -> Result<()> {
    let _raw_guard = RawModeGuard::new()?;
    let mut stdout = io::stdout();
    let mut buffer = String::new();
    render_prompt(&mut stdout, &buffer)?;

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    disable_raw_mode()?;
                    println!();
                    break;
                }
                KeyCode::Char('/') => {
                    disable_raw_mode()?;
                    println!();
                    show_main_menu()?;
                    enable_raw_mode()?;
                    render_prompt(&mut stdout, &buffer)?;
                }
                KeyCode::Char(ch) => {
                    buffer.push(ch);
                    render_prompt(&mut stdout, &buffer)?;
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    render_prompt(&mut stdout, &buffer)?;
                }
                KeyCode::Enter => {
                    println!();
                    buffer.clear();
                    render_prompt(&mut stdout, &buffer)?;
                }
                KeyCode::Esc => {
                    buffer.clear();
                    render_prompt(&mut stdout, &buffer)?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn show_main_menu() -> Result<()> {
    let items = ["explore", "create", "insert", "search"];
    println!();
    let selection = Select::new()
        .with_prompt("Menu")
        .items(&items)
        .default(0)
        .interact()?;

    match items[selection] {
        "explore" => show_market_menu(),
        "create" => {
            println!("TODO: create menu is not implemented yet.");
            Ok(())
        }
        "insert" => {
            println!("TODO: insert menu is not implemented yet.");
            Ok(())
        }
        "search" => {
            println!("TODO: search menu is not implemented yet.");
            Ok(())
        }
        _ => Ok(()),
    }
}

fn show_market_menu() -> Result<()> {
    loop {
        let query = match read_line_or_esc("Market query")? {
            Some(value) => value,
            None => return Ok(()),
        };

        if query.trim().is_empty() {
            return Ok(());
        }

        let results = mock_market_results(&query, 12);
        let display_results = truncate_results(&results)?;
        let mut selected_index = 0;

        loop {
            let selection = Select::new()
                .with_prompt("Market results")
                .items(&display_results)
                .max_length(3)
                .default(selected_index)
                .interact_opt()?;

            match selection {
                Some(index) => {
                    // ここは display_results と results の index が一致している前提
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
                    pb.set_message("Loading...");
                    pb.enable_steady_tick(Duration::from_millis(80));

                    sleep(Duration::from_secs(3));

                    pb.finish_and_clear();
                    println!("Selected: {}\n", results[index]);

                    selected_index = index;
                    continue;
                }
                None => {
                    println!();
                    break;
                }
            }
        }
    }
}

fn mock_market_results(query: &str, count: usize) -> Vec<String> {
    (1..=count)
        .map(|idx| format!("{query} result #{idx}"))
        .collect()
}

fn truncate_results(results: &[String]) -> Result<Vec<String>> {
    let (cols, _) = crossterm::terminal::size()?;
    let max_width = cols.saturating_sub(6) as usize;
    if max_width == 0 {
        return Ok(results.to_vec());
    }
    Ok(results
        .iter()
        .map(|item| truncate_to_width(item, max_width))
        .collect())
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        return text.to_string();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let mut trimmed = text.chars().take(max_width - 3).collect::<String>();
    trimmed.push_str("...");
    trimmed
}

fn render_prompt(stdout: &mut io::Stdout, buffer: &str) -> Result<()> {
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::CurrentLine)
    )?;
    write!(stdout, "kinic> {buffer}")?;
    stdout.flush()?;
    Ok(())
}

fn read_line_or_esc(prompt: &str) -> Result<Option<String>> {
    let was_raw = crossterm::terminal::is_raw_mode_enabled()?;
    if !was_raw {
        enable_raw_mode()?;
    }
    let mut stdout = io::stdout();
    println!();
    print!("{prompt}: ");
    stdout.flush()?;
    let mut input = String::new();

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => {
                    write!(stdout, "\r\n")?;
                    stdout.flush()?;
                    break;
                }
                KeyCode::Esc => {
                    write!(stdout, "\r\n")?;
                    stdout.flush()?;
                    if !was_raw {
                        disable_raw_mode()?;
                    }
                    return Ok(None);
                }
                KeyCode::Backspace => {
                    if input.pop().is_some() {
                        execute!(stdout, cursor::MoveLeft(1), Clear(ClearType::UntilNewLine))?;
                        stdout.flush()?;
                    }
                }
                KeyCode::Char(ch) => {
                    input.push(ch);
                    write!(stdout, "{ch}")?;
                    stdout.flush()?;
                }
                _ => {}
            }
        }
    }

    if !was_raw {
        disable_raw_mode()?;
    }
    Ok(Some(input))
}

struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}
