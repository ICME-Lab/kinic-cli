use anyhow::Result;
use std::io::{self, Write};

use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use std::fs;
use std::path::Path;
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
    let samples = load_samples("samples")?;
    run_menu(samples)
}

fn run_menu(samples: Vec<SampleItem>) -> Result<()> {
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
                    show_main_menu(&samples)?;
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

fn show_main_menu(samples: &[SampleItem]) -> Result<()> {
    let items = ["explore", "create", "insert", "search"];
    println!();
    let selection = Select::new()
        .with_prompt("Menu")
        .items(&items)
        .default(0)
        .interact()?;

    match items[selection] {
        "explore" => show_market_menu(samples),
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

fn show_market_menu(samples: &[SampleItem]) -> Result<()> {
    loop {
        let query = match read_line_or_esc("Explore query")? {
            Some(value) => value,
            None => return Ok(()),
        };

        let query = query.trim().to_string();
        if query.is_empty() {
            return Ok(());
        }

        let matches = search_samples(samples, &query);
        if matches.is_empty() {
            println!("No results.\n");
            continue;
        }

        let results: Vec<String> = matches
            .iter()
            .map(|item| format!("[$0.01] {}", item.query))
            .collect();
        let display_results = truncate_results(&results)?;
        let mut selected_index = 0;

        loop {
            let selection = Select::new()
                .with_prompt("\nQuery")
                .items(&display_results)
                .max_length(10)
                .default(selected_index)
                .interact_opt()?;

            match selection {
                Some(index) => {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
                    pb.set_message("Loading...");
                    pb.enable_steady_tick(Duration::from_millis(80));

                    sleep(Duration::from_secs(3));

                    pb.finish_and_clear();
                    println!("Premise: {}", matches[index].premise);
                    println!("Knowledge: {}", matches[index].knowledge);
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

#[derive(Deserialize)]
struct SampleFile {
    items: Vec<SampleItem>,
}

#[derive(Clone, Deserialize)]
struct SampleItem {
    premise: String,
    query: String,
    knowledge: String,
}

fn load_samples<P: AsRef<Path>>(dir: P) -> Result<Vec<SampleItem>> {
    let mut items = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let payload = fs::read_to_string(&path)?;
        let file: SampleFile = serde_json::from_str(&payload)?;
        items.extend(file.items);
    }
    Ok(items)
}

fn search_samples<'a>(samples: &'a [SampleItem], query: &str) -> Vec<&'a SampleItem> {
    let needle = query.to_lowercase();
    let mut scored: Vec<(i64, &SampleItem)> = samples
        .iter()
        .filter_map(|item| {
            let q_score = fuzzy_score(&needle, &item.query.to_lowercase());
            let p_score = fuzzy_score(&needle, &item.premise.to_lowercase());
            let score = q_score.max(p_score)?;
            Some((score, item))
        })
        .collect();

    scored.sort_by(|(a_score, a_item), (b_score, b_item)| {
        b_score
            .cmp(a_score)
            .then_with(|| a_item.query.cmp(&b_item.query))
    });

    scored.into_iter().map(|(_, item)| item).collect()
}

fn fuzzy_score(needle: &str, haystack: &str) -> Option<i64> {
    if needle.is_empty() {
        return Some(0);
    }
    let mut score: i64 = 0;
    let mut last_idx: i64 = -1;
    let mut hi_iter = haystack.char_indices();
    for ch in needle.chars() {
        let mut found = None;
        for (idx, hch) in hi_iter.by_ref() {
            if hch == ch {
                found = Some(idx as i64);
                break;
            }
        }
        let idx = found?;
        if last_idx >= 0 {
            let gap = idx - last_idx - 1;
            score -= gap;
            if gap == 0 {
                score += 3;
            }
        } else {
            score += 5;
        }
        last_idx = idx;
    }
    score += 1;
    Some(score)
}
