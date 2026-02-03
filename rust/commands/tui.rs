use anyhow::{anyhow, Context, Result};
use std::io::{self, Write};

use headless_chrome::{Browser, LaunchOptionsBuilder};
use headless_chrome::browser::tab::RequestPausedDecision;
use headless_chrome::protocol::cdp::{Fetch, Network};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Deserialize;
use serde_json::Value;
use std::ffi::OsStr;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;
use tokio::runtime::Handle;
use tokio::task::block_in_place;
use url::Url;
use crossterm::style::{Color, Stylize};
use crossterm::terminal;
use unicode_width::UnicodeWidthChar;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};
use cliclack::select;

use crate::commands::CommandContext;

pub async fn handle(_ctx: &CommandContext) -> Result<()> {
    let samples = load_samples("samples")?;
    run_menu(samples)
}

fn run_menu(samples: Vec<SampleItem>) -> Result<()> {
    let _raw_guard = RawModeGuard::new()?;
    let mut stdout = io::stdout();
    let mut buffer = String::new();
    print_startup_banner(&mut stdout)?;
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
                    let query = buffer.trim().to_string();
                    println!();
                    buffer.clear();
                    if !query.is_empty() {
                        disable_raw_mode()?;
                        show_market_menu(&samples, Some(query))?;
                        enable_raw_mode()?;
                    }
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
    println!();
    let selection = select("Menu")
        .item("explore", "explore", "")
        .item("create", "create", "")
        .item("insert", "insert", "")
        .item("search", "search", "")
        .interact();

    match selection {
        Ok("explore") => show_market_menu(samples, None),
        Ok("create") => {
            println!("TODO: create menu is not implemented yet.");
            Ok(())
        }
        Ok("insert") => show_insert_menu(),
        Ok("search") => {
            println!("TODO: search menu is not implemented yet.");
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::Interrupted => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn show_insert_menu() -> Result<()> {
    println!();
    let selection = select("Insert")
        .item("url", "url", "")
        .item("markdown", "markdown", "")
        .item("conversation", "conversation", "")
        .interact();

    match selection {
        Ok("url") => handle_insert_url(),
        Ok("markdown") => {
            println!("TODO: insert markdown is not implemented yet.");
            Ok(())
        }
        Ok("conversation") => {
            println!("TODO: insert conversation is not implemented yet.");
            Ok(())
        }
        Ok(_) => Ok(()),
        Err(err) if err.kind() == ErrorKind::Interrupted => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn show_market_menu(samples: &[SampleItem], initial_query: Option<String>) -> Result<()> {
    let mut combined = Vec::new();
    combined.extend(samples.iter().cloned().map(|item| ExploreEntry {
        source: Source::Sample,
        item,
    }));

    let mut pending_query = initial_query;
    loop {
        let memories = load_samples("memories").unwrap_or_default();
        combined.truncate(samples.len());
        combined.extend(memories.into_iter().map(|item| ExploreEntry {
            source: Source::Memory,
            item,
        }));

        let query = match pending_query.take() {
            Some(value) => value,
            None => match read_line_or_esc("Explore query")? {
                Some(value) => value,
                None => {
                    return Ok(());
                }
            },
        };

        let query = query.trim().to_string();
        if query.is_empty() {
            return Ok(());
        }

        let matches = search_samples(&combined, &query);
        if matches.is_empty() {
            println!("No results.\n");
            continue;
        }

        let results: Vec<String> = matches
            .iter()
            .map(|entry| {
                let price = match entry.source {
                    Source::Sample => "[$0.01]",
                    Source::Memory => "[$----]",
                };
                format!("{price} {}", entry.item.query)
            })
            .collect();
        let prices: Vec<f32> = matches
            .iter()
            .map(|entry| match entry.source {
                Source::Sample => 0.01,
                Source::Memory => 0.0,
            })
            .collect();
        let display_results = truncate_results(&results)?;
        let mut selected_index = 0;
        let mut view_start = 0usize;
        let mut selected_flags = vec![false; display_results.len()];

        loop {
            let selection = select_list_multi(
                "✨Knowledges",
                &display_results,
                &mut selected_index,
                &mut view_start,
                10,
                &mut selected_flags,
                &prices,
            )?;

            match selection {
                Some(indices) => {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
                    pb.set_message("Loading...");
                    pb.enable_steady_tick(Duration::from_millis(80));

                    sleep(Duration::from_secs(1));

                    pb.finish_and_clear();
                    for index in indices {
                        print_boxed_entry(
                            &matches[index].item.query,
                            &matches[index].item.premise,
                            &matches[index].item.knowledge,
                        );
                    }
                    for flag in &mut selected_flags {
                        *flag = false;
                    }
                    continue;
                }
                None => {
                    println!();
                    return Ok(());
                }
            }
        }
    }
}

const READABILITY_JS: &str = include_str!("readability.js");

fn handle_insert_url() -> Result<()> {
    let raw_url = match read_line_or_esc("URL")? {
        Some(value) => value,
        None => return Ok(()),
    };
    let raw_url = raw_url.trim();
    if raw_url.is_empty() {
        return Ok(());
    }

    let url = Url::parse(raw_url).context("Invalid URL")?;
    let (html, readable_html) = fetch_html_with_headless_chrome(&url)?;
    let body_html = extract_body_html(&html)?;
    let readable_html = match readable_html {
        Some(content) => content,
        None => {
            eprintln!("Readability JS failed, falling back to body HTML");
            body_html
        }
    };
    let markdown = htmd::convert(&readable_html).context("Failed to convert HTML to Markdown")?;
    let prompt = fs::read_to_string("prompt.md").context("Failed to read prompt.md")?;
    let output = call_openai(&prompt, &markdown)?;
    save_llm_output(&url, &output)?;
    Ok(())
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

fn select_list_multi(
    prompt: &str,
    items: &[String],
    selected_index: &mut usize,
    view_start: &mut usize,
    max_rows: usize,
    selected_flags: &mut Vec<bool>,
    prices: &[f32],
) -> Result<Option<Vec<usize>>> {
    if items.is_empty() {
        return Ok(None);
    }
    let _raw_guard = RawModeGuard::new()?;
    let mut stdout = io::stdout();
    if selected_flags.len() != items.len() {
        selected_flags.clear();
        selected_flags.resize(items.len(), false);
    }
    let mut selected = (*selected_index).min(items.len().saturating_sub(1));
    let view_height = max_rows.max(1);
    let max_start = items.len().saturating_sub(view_height);
    if *view_start > max_start {
        *view_start = max_start;
    }
    let mut view_start_local = *view_start;
    let mut lines_rendered = 0usize;

    let mut multi_mode = selected_flags.iter().any(|flag| *flag);
    render_list_multi(
        &mut stdout,
        prompt,
        items,
        selected,
        selected_flags,
        view_start_local,
        view_height,
        multi_mode,
        prices,
        &mut lines_rendered,
    )?;

    loop {
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(event::KeyModifiers::CONTROL) => {
                    *view_start = view_start_local;
                    clear_rendered(&mut stdout, lines_rendered)?;
                    return Ok(None);
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if selected > 0 {
                        selected -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if selected + 1 < items.len() {
                        selected += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    if let Some(flag) = selected_flags.get_mut(selected) {
                        *flag = !*flag;
                    }
                    multi_mode = selected_flags.iter().any(|flag| *flag);
                }
                KeyCode::Left => {
                    for flag in selected_flags.iter_mut() {
                        *flag = false;
                    }
                    multi_mode = false;
                }
                KeyCode::Enter => {
                    *view_start = view_start_local;
                    *selected_index = selected;
                    clear_rendered(&mut stdout, lines_rendered)?;
                    let mut chosen: Vec<usize> = selected_flags
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, flag)| if *flag { Some(idx) } else { None })
                        .collect();
                    if chosen.is_empty() && !multi_mode {
                        chosen.push(selected);
                    }
                    return Ok(Some(chosen));
                }
                KeyCode::Esc => {
                    *view_start = view_start_local;
                    *selected_index = selected;
                    clear_rendered(&mut stdout, lines_rendered)?;
                    return Ok(None);
                }
                _ => {}
            }

            if selected < view_start_local {
                view_start_local = selected;
            } else if selected >= view_start_local + view_height {
                view_start_local = selected.saturating_sub(view_height - 1);
            }

            render_list_multi(
                &mut stdout,
                prompt,
                items,
                selected,
                selected_flags,
                view_start_local,
                view_height,
                multi_mode,
                prices,
                &mut lines_rendered,
            )?;
        }
    }
}

fn render_list_multi(
    stdout: &mut io::Stdout,
    prompt: &str,
    items: &[String],
    selected: usize,
    selected_flags: &[bool],
    view_start: usize,
    view_height: usize,
    multi_mode: bool,
    prices: &[f32],
    lines_rendered: &mut usize,
) -> Result<()> {
    if *lines_rendered > 0 {
        execute!(stdout, cursor::MoveUp(*lines_rendered as u16))?;
    }
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::FromCursorDown)
    )?;

    let sum_selected: f32 = selected_flags
        .iter()
        .enumerate()
        .filter_map(|(idx, flag)| {
            if *flag {
                prices.get(idx).copied()
            } else {
                None
            }
        })
        .sum();
    let sum = if !multi_mode {
        prices.get(selected).copied().unwrap_or(0.0)
    } else {
        sum_selected
    };
    write!(
        stdout,
        "{} ({}/{}) Cart ${:.2}\r\n",
        prompt,
        selected.saturating_add(1),
        items.len(),
        sum
    )?;
    let mut count = 1usize;
    let end = (view_start + view_height).min(items.len());
    for (idx, label) in items.iter().enumerate().take(end).skip(view_start) {
        let is_selected = selected_flags.get(idx).copied().unwrap_or(false);
        if idx == selected {
            let dot = if is_selected {
                "●".with(Color::White)
            } else if multi_mode {
                "○".with(Color::Cyan)
            } else {
                "●".with(Color::Cyan)
            };
            let text = label.clone().with(Color::Cyan);
            write!(stdout, "{} {}", dot, text)?;
        } else if is_selected {
            let dot = "●".with(Color::White);
            let text = label.clone().with(Color::Grey);
            write!(stdout, "{} {}", dot, text)?;
        } else {
            let dot = "○".with(Color::DarkGrey);
            let text = label.clone().with(Color::Grey).dim();
            write!(stdout, "{} {}", dot, text)?;
        }
        execute!(stdout, Clear(ClearType::UntilNewLine))?;
        write!(stdout, "\r\n")?;
        count += 1;
    }
    let hint = "Enter: purchase  |  Space: multi-select  |  ←: unselect all";
    write!(stdout, "{}", hint.with(Color::DarkGrey))?;
    execute!(stdout, Clear(ClearType::UntilNewLine))?;
    write!(stdout, "\r\n")?;
    count += 1;
    stdout.flush()?;
    *lines_rendered = count;
    Ok(())
}

fn clear_rendered(stdout: &mut io::Stdout, lines_rendered: usize) -> Result<()> {
    if lines_rendered > 0 {
        execute!(
            stdout,
            cursor::MoveUp(lines_rendered as u16),
            cursor::MoveToColumn(0),
            Clear(ClearType::FromCursorDown)
        )?;
        stdout.flush()?;
    }
    execute!(stdout, cursor::Show)?;
    stdout.flush()?;
    Ok(())
}

fn render_prompt(stdout: &mut io::Stdout, buffer: &str) -> Result<()> {
    execute!(
        stdout,
        cursor::MoveToColumn(0),
        Clear(ClearType::CurrentLine)
    )?;
    let prompt = "kinic > ";
    write!(stdout, "{prompt}{buffer}")?;
    write!(stdout, "\r\n")?;
    write!(
        stdout,
        "{}",
        " / Menu  |  Enter Search  |  Esc Clear".with(Color::DarkGrey)
    )?;
    let prompt_width = display_width(prompt);
    let buffer_width = display_width(buffer);
    execute!(stdout, cursor::MoveUp(1))?;
    execute!(stdout, cursor::MoveToColumn((prompt_width + buffer_width) as u16))?;
    stdout.flush()?;
    Ok(())
}

fn print_boxed_entry(query: &str, premise: &str, knowledge: &str) {
    let content_width = terminal::size()
        .map(|(cols, _rows): (u16, u16)| (cols / 2).saturating_sub(4) as usize)
        .unwrap_or(80)
        .max(20);

    let mut lines = Vec::new();
    lines.extend(wrap_plain(query, content_width, LineStyle::Query));
    lines.push(StyledLine {
        text: String::new(),
        style: LineStyle::Query,
    });
    lines.extend(wrap_plain(premise, content_width, LineStyle::Premise));
    lines.push(StyledLine {
        text: String::new(),
        style: LineStyle::Premise,
    });
    lines.extend(wrap_plain(knowledge, content_width, LineStyle::Knowledge));

    let width = lines
        .iter()
        .map(|line| display_width(&line.text))
        .max()
        .unwrap_or(0);

    let top = format!("╭{}╮", "─".repeat(width + 2));
    let bottom = format!("╰{}╯", "─".repeat(width + 2));
    println!("{top}");
    for line in &lines {
        let pad = width.saturating_sub(display_width(&line.text));
        match line.style {
            LineStyle::Query => println!(" {}{}", line.text.clone().bold(), " ".repeat(pad)),
            LineStyle::Premise => println!(" {}{}", line.text.clone().dim(), " ".repeat(pad)),
            LineStyle::Knowledge => println!(" {}{}", line.text, " ".repeat(pad)),
        }
    }
    println!("{bottom}\n");
}

#[derive(Copy, Clone)]
enum LineStyle {
    Query,
    Premise,
    Knowledge,
}

struct StyledLine {
    text: String,
    style: LineStyle,
}

fn wrap_plain(text: &str, width: usize, style: LineStyle) -> Vec<StyledLine> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;

    for word in text.split_whitespace() {
        let word_len = display_width(word);
        let extra = if current_len > 0 { 1 } else { 0 };
        if current_len + extra + word_len <= width {
            if current_len > 0 {
                current.push(' ');
                current_len += 1;
            }
            current.push_str(word);
            current_len += word_len;
            continue;
        }

        if !current.is_empty() {
            lines.push(StyledLine {
                text: current,
                style,
            });
            current = String::new();
        }

        if word_len > width {
            let mut chunk = String::new();
            let mut chunk_len = 0usize;
            for ch in word.chars() {
                let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                if chunk_len + ch_width > width {
                    lines.push(StyledLine {
                        text: chunk,
                        style,
                    });
                    chunk = String::new();
                    chunk_len = 0;
                }
                chunk.push(ch);
                chunk_len += ch_width;
            }
            current = chunk;
            current_len = chunk_len;
        } else {
            current.push_str(word);
            current_len = word_len;
        }
    }

    if !current.is_empty() {
        lines.push(StyledLine {
            text: current,
            style,
        });
    }

    lines
}

fn display_width(text: &str) -> usize {
    text.chars()
        .map(|ch| UnicodeWidthChar::width(ch).unwrap_or(0))
        .sum()
}

fn print_startup_banner(stdout: &mut io::Stdout) -> Result<()> {
    let logo = include_str!("../../logo.txt");
    print_gradient_logo(stdout, logo)?;
    let blurb = "🧠 Kinic is a platform that turns human-generated “answers,” “judgment,” and “expertise” into AI tradable knowledge via micropayments (small, per-use payments)—and makes it usable as AI-ready world knowledge.\n🧩 From individual expertise to full AI workflows, you can sell valuable outputs in small units 🌍 so people everywhere can buy them, build on them, and collectively compound what’s possible 📈.";
    print_wrapped_rich_text(
        stdout,
        blurb,
        Color::DarkGrey,
        Color::White,
        Some(80),
        true,
        0,
    )?;
    writeln!(stdout)?;
    let tips = "Tips for getting started:\n1. Search for world knowledge.\n2. Import your knowledge to build a personalized AI memory.\n3. Add your expertise to the world knowledge.";
    print_wrapped_rich_text(stdout, tips, Color::White, Color::White, Some(80), false, 2)?;
    write!(
        stdout,
        "\r\n"
    )?;
    stdout.flush()?;
    Ok(())
}

fn print_gradient_logo(stdout: &mut io::Stdout, logo: &str) -> Result<()> {
    let lines: Vec<&str> = logo.lines().collect();
    for line in lines {
        let chars: Vec<char> = line.chars().collect();
        let width = chars.len().max(1);
        for (idx, ch) in chars.into_iter().enumerate() {
            let t = if width <= 1 {
                0.0
            } else {
                idx as f32 / (width - 1) as f32
            };
            let (r, g, b) = pink_gradient(t);
            write!(stdout, "{}", ch.with(Color::Rgb { r, g, b }))?;
        }
        write!(stdout, "\r\n")?;
    }
    write!(stdout, "\r\n")?;
    Ok(())
}

fn pink_gradient(t: f32) -> (u8, u8, u8) {
    let start = (255.0_f32, 105.0_f32, 180.0_f32);
    let end = (255.0_f32, 20.0_f32, 147.0_f32);
    let r = start.0 + (end.0 - start.0) * t;
    let g = start.1 + (end.1 - start.1) * t;
    let b = start.2 + (end.2 - start.2) * t;
    (r.round() as u8, g.round() as u8, b.round() as u8)
}

#[derive(Clone)]
struct RichSpan {
    text: String,
    bold: bool,
}

fn parse_bold_spans(text: &str) -> Vec<RichSpan> {
    let mut spans = Vec::new();
    let mut bold = false;
    for part in text.split("**") {
        if !part.is_empty() {
            spans.push(RichSpan {
                text: part.to_string(),
                bold,
            });
        }
        bold = !bold;
    }
    spans
}

fn print_wrapped_rich_text(
    stdout: &mut io::Stdout,
    text: &str,
    normal_color: Color,
    bold_color: Color,
    max_width: Option<usize>,
    paragraph_gap: bool,
    indent: usize,
) -> Result<()> {
    let terminal_width = terminal::size()
        .map(|(cols, _rows): (u16, u16)| cols.saturating_sub(4) as usize)
        .unwrap_or(80)
        .max(30);
    let content_width = max_width
        .map(|limit| terminal_width.min(limit))
        .unwrap_or(terminal_width);
    let content_width = content_width.saturating_sub(indent);
    for (idx, paragraph) in text.split('\n').enumerate() {
        if paragraph_gap && idx > 0 {
            write!(stdout, "\r\n")?;
        }
        if indent > 0 {
            write!(stdout, "{}", " ".repeat(indent))?;
        }
        let spans = parse_bold_spans(paragraph);
        let mut line = Vec::<RichSpan>::new();
        let mut line_width = 0usize;

        for span in spans {
            let mut buf = String::new();
            for ch in span.text.chars() {
                if ch.is_whitespace() {
                    flush_word(
                        stdout,
                        &mut line,
                        &mut line_width,
                        &buf,
                        span.bold,
                        content_width,
                        normal_color,
                        bold_color,
                    )?;
                    buf.clear();
                } else {
                    buf.push(ch);
                }
            }
            flush_word(
                stdout,
                &mut line,
                &mut line_width,
                &buf,
                span.bold,
                content_width,
                normal_color,
                bold_color,
            )?;
        }

        write_rich_line(stdout, &line, normal_color, bold_color)?;
        write!(stdout, "\r\n")?;
    }
    Ok(())
}

fn flush_word(
    stdout: &mut io::Stdout,
    line: &mut Vec<RichSpan>,
    line_width: &mut usize,
    word: &str,
    bold: bool,
    max_width: usize,
    normal_color: Color,
    bold_color: Color,
) -> Result<()> {
    if word.is_empty() {
        return Ok(());
    }
    let word_width = display_width(word);
    let extra = if *line_width > 0 { 1 } else { 0 };
    if *line_width + extra + word_width > max_width && !line.is_empty() {
        write_rich_line(stdout, line, normal_color, bold_color)?;
        write!(stdout, "\r\n")?;
        line.clear();
        *line_width = 0;
    }
    if *line_width > 0 {
        line.push(RichSpan {
            text: " ".to_string(),
            bold: false,
        });
        *line_width += 1;
    }
    line.push(RichSpan {
        text: word.to_string(),
        bold,
    });
    *line_width += word_width;
    Ok(())
}

fn write_rich_line(
    stdout: &mut io::Stdout,
    line: &[RichSpan],
    normal_color: Color,
    bold_color: Color,
) -> Result<()> {
    for span in line {
        if span.bold {
            write!(
                stdout,
                "{}",
                span.text.clone().with(bold_color).bold()
            )?;
        } else {
            write!(stdout, "{}", span.text.clone().with(normal_color))?;
        }
    }
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

fn fetch_html_with_headless_chrome(url: &Url) -> Result<(String, Option<String>)> {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(true)
            .args(vec![OsStr::new("--blink-settings=imagesEnabled=false")])
            .build()
            .context("Failed to build launch options")?,
    )
    .context("Failed to launch headless Chrome")?;

    let tab = browser.new_tab().context("Failed to open new tab")?;
    let patterns = [Fetch::RequestPattern {
        url_pattern: None,
        resource_Type: Some(Network::ResourceType::Stylesheet),
        request_stage: Some(Fetch::RequestStage::Request),
    }];
    tab.enable_fetch(Some(&patterns), Some(false))
        .context("Failed to enable fetch interception")?;
    tab.enable_request_interception(Arc::new(
        |_transport, _session_id, event: Fetch::events::RequestPausedEvent| {
            if event.params.resource_Type == Network::ResourceType::Stylesheet {
                RequestPausedDecision::Fail(Fetch::FailRequest {
                    request_id: event.params.request_id,
                    error_reason: Network::ErrorReason::BlockedByClient,
                })
            } else {
                RequestPausedDecision::Continue(None)
            }
        },
    ))
    .context("Failed to enable request interception")?;
    tab.navigate_to(url.as_str())
        .context("Failed to navigate to URL")?;
    wait_for_page_ready(&tab, Duration::from_secs(10))
        .context("Failed while waiting for page navigation")?;
    let html = tab.get_content().context("Failed to read page HTML")?;
    let readability_html = extract_readable_html_from_tab(&tab).ok();
    let _ = tab.close(true);
    Ok((html, readability_html))
}

fn extract_readable_html_from_tab(tab: &headless_chrome::Tab) -> Result<String> {
    let js = r#"
(() => {
  try {
    const html = document.documentElement.outerHTML;
    const doc = new DOMParser().parseFromString(html, "text/html");
    const article = new Readability(doc).parse();
    return article && article.content ? article.content : "";
  } catch (e) {
    return "";
  }
})()
"#;
    tab.evaluate(READABILITY_JS, false)
        .context("Failed to inject Readability.js")?;
    let result = tab
        .evaluate(js, true)
        .context("Failed to execute Readability")?;
    let content = result
        .value
        .and_then(|value| value.as_str().map(|s| s.to_string()))
        .unwrap_or_default();
    if content.trim().is_empty() {
        return Err(anyhow!("Readability JS returned empty content"));
    }
    Ok(content)
}

fn wait_for_page_ready(tab: &headless_chrome::Tab, timeout: Duration) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        let ready = tab
            .evaluate(
                r##"
(() => {
  const rs = document.readyState;
  const hasBody = !!document.body;
  const hasContent = !!document.querySelector("#postBody");
  return rs === "complete" || (hasBody && hasContent);
})()
"##,
                true,
            )
            .ok()
            .and_then(|v| v.value)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if ready {
            return Ok(());
        }
        sleep(Duration::from_millis(200));
    }
    Err(anyhow!("Timed out waiting for page readiness"))
}

fn extract_body_html(html: &str) -> Result<String> {
    let lower = html.to_lowercase();
    let body_open = lower
        .find("<body")
        .ok_or_else(|| anyhow!("No <body> tag found"))?;
    let body_tag_end = lower[body_open..]
        .find('>')
        .ok_or_else(|| anyhow!("No closing > for <body> tag"))?
        + body_open
        + 1;
    let body_close = lower
        .rfind("</body>")
        .ok_or_else(|| anyhow!("No </body> tag found"))?;
    if body_close <= body_tag_end {
        return Err(anyhow!("Malformed <body> tags"));
    }
    Ok(html[body_tag_end..body_close].to_string())
}

fn call_openai(prompt: &str, markdown: &str) -> Result<String> {
    let prompt = prompt.to_string();
    let markdown = markdown.to_string();
    block_in_place(|| {
        Handle::current().block_on(async move {
            let api_key =
                std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY is not set")?;
            let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4.1".to_string());
            let input_text = format!("{prompt}\nInput\n>>>{markdown}<<<");

            let client = reqwest::Client::new();
            let response = client
                .post("https://api.openai.com/v1/responses")
                .bearer_auth(api_key)
                .json(&serde_json::json!({
                    "model": model,
                    "input": input_text
                }))
                .send()
                .await
                .context("Failed to send request to OpenAI")?;

            let status = response.status();
            let body = response
                .text()
                .await
                .context("Failed to read OpenAI response")?;
            if !status.is_success() {
                return Err(anyhow!("OpenAI API error: {status} {body}"));
            }

            let value: Value =
                serde_json::from_str(&body).context("Failed to parse OpenAI response")?;
            if let Some(text) = value.get("output_text").and_then(|v| v.as_str()) {
                return Ok(text.to_string());
            }

            let mut parts = Vec::new();
            if let Some(items) = value.get("output").and_then(|v| v.as_array()) {
                for item in items {
                    if item.get("type").and_then(|v| v.as_str()) != Some("message") {
                        continue;
                    }
                    if let Some(content) = item.get("content").and_then(|v| v.as_array()) {
                        for piece in content {
                            if piece.get("type").and_then(|v| v.as_str())
                                == Some("output_text")
                            {
                                if let Some(text) = piece.get("text").and_then(|v| v.as_str()) {
                                    parts.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }

            if parts.is_empty() {
                Err(anyhow!("OpenAI response did not contain output text"))
            } else {
                Ok(parts.join(""))
            }
        })
    })
}

fn save_llm_output(url: &Url, output: &str) -> Result<()> {
    let value: Value =
        serde_json::from_str(output).context("LLM output is not valid JSON")?;
    fs::create_dir_all("memories").context("Failed to create memories directory")?;
    let slug = slugify_url(url);
    let path = next_available_path(&slug)?;
    let payload = serde_json::to_string_pretty(&value).context("Failed to format JSON")?;
    fs::write(&path, payload).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn next_available_path(slug: &str) -> Result<std::path::PathBuf> {
    let base = std::path::PathBuf::from("memories").join(format!("{slug}.json"));
    if !base.exists() {
        return Ok(base);
    }
    for idx in 2..=9999 {
        let candidate = std::path::PathBuf::from("memories").join(format!("{slug}-{idx}.json"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(anyhow!("Failed to find available filename for {slug}"))
}

fn slugify_url(url: &Url) -> String {
    let mut parts = Vec::new();
    if let Some(host) = url.host_str() {
        parts.push(host.to_string());
    }
    let path = url
        .path_segments()
        .map(|segs| segs.collect::<Vec<_>>())
        .unwrap_or_default();
    for seg in path {
        if !seg.is_empty() {
            parts.push(seg.to_string());
        }
    }
    let raw = if parts.is_empty() {
        "memory".to_string()
    } else {
        parts.join("-")
    };
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.chars() {
        let c = ch.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "memory".to_string()
    } else {
        trimmed
    }
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
        let mut stdout = io::stdout();
        let _ = execute!(stdout, cursor::Show);
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

#[derive(Clone, Copy)]
enum Source {
    Sample,
    Memory,
}

#[derive(Clone)]
struct ExploreEntry {
    source: Source,
    item: SampleItem,
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

fn search_samples<'a>(samples: &'a [ExploreEntry], query: &str) -> Vec<&'a ExploreEntry> {
    let needle = query.to_lowercase();
    let mut scored: Vec<(i64, &ExploreEntry)> = samples
        .iter()
        .filter_map(|entry| {
            let q_score = fuzzy_score(&needle, &entry.item.query.to_lowercase());
            let p_score = fuzzy_score(&needle, &entry.item.premise.to_lowercase());
            let score = q_score.max(p_score)?;
            Some((score, entry))
        })
        .collect();

    scored.sort_by(|(a_score, a_item), (b_score, b_item)| {
        b_score
            .cmp(a_score)
            .then_with(|| a_item.item.query.cmp(&b_item.item.query))
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
