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
        let display_results = truncate_results(&results)?;
        let mut selected_index = 0;
        let mut view_start = 0usize;
        let mut selected_flags = vec![false; display_results.len()];

        loop {
            let selection = select_list_multi(
                "Query",
                &display_results,
                &mut selected_index,
                &mut view_start,
                10,
                &mut selected_flags,
            )?;

            match selection {
                Some(indices) => {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(ProgressStyle::with_template("{spinner} {msg}").unwrap());
                    pb.set_message("Loading...");
                    pb.enable_steady_tick(Duration::from_millis(80));

                    sleep(Duration::from_secs(3));

                    pb.finish_and_clear();
                    for index in indices {
                        println!("Premise: {}", matches[index].item.premise);
                        println!("Knowledge: {}", matches[index].item.knowledge);
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

    write!(stdout, "{prompt}\r\n")?;
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
    write!(stdout, "kinic > {buffer}")?;
    stdout.flush()?;
    Ok(())
}

fn print_startup_banner(stdout: &mut io::Stdout) -> Result<()> {
    let pink = Color::Rgb { r: 255, g: 105, b: 180 };
    let yellow = Color::Yellow;
    let cyan = Color::Cyan;
    let orange = Color::Rgb { r: 255, g: 165, b: 0 };

    let k = ["K   K", "K  K ", "K K  ", "K  K ", "K   K"];
    let i = [" III ", "  I  ", "  I  ", "  I  ", " III "];
    let n = ["N   N", "NN  N", "N N N", "N  NN", "N   N"];
    let c = [" CCCC", "C    ", "C    ", "C    ", " CCCC"];

    for idx in 0..k.len() {
        write!(
            stdout,
            "{} {} {} {} {}\r\n",
            k[idx].with(pink),
            i[idx].with(yellow),
            n[idx].with(cyan),
            i[idx].with(yellow),
            c[idx].with(orange),
        )?;
    }
    write!(stdout, "\r\n")?;
    write!(
        stdout,
        "{}\r\n\r\n",
        " / Menu  |  Enter Search  |  Esc Clear".with(Color::DarkGrey)
    )?;
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
