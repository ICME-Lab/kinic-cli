use std::{path::Path, process::Command};

use anyhow::{Context, Result};
use gag::Gag;
use pdf_extract::extract_text;

use crate::cli::ConvertPdfArgs;

pub async fn handle(args: ConvertPdfArgs) -> Result<()> {
    let markdown = pdf_to_markdown(&args.file_path)?;
    println!("{markdown}");
    Ok(())
}

pub fn pdf_to_markdown(path: &Path) -> Result<String> {
    extract_with_pdftotext(path).or_else(|primary_err| {
        extract_with_pdf_extract_quiet(path).with_context(|| {
            format!(
                "Failed to extract text from {} using pdftotext ({primary_err}) and pdf-extract",
                path.display()
            )
        })
    })
}

fn extract_with_pdftotext(path: &Path) -> Result<String> {
    let output = Command::new("pdftotext")
        .arg("-layout")
        .arg("-enc")
        .arg("UTF-8")
        .arg(path)
        .arg("-")
        .output()
        .with_context(|| "Failed to spawn pdftotext; ensure poppler is installed")?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "pdftotext exited with status {}",
            output.status
        ));
    }

    String::from_utf8(output.stdout)
        .with_context(|| "pdftotext output was not valid UTF-8")
}

fn extract_with_pdf_extract_quiet(path: &Path) -> Result<String> {
    // Suppress noisy stdout/stderr from the pdf-extract parser.
    let _gag_out = Gag::stdout().ok();
    let _gag_err = Gag::stderr().ok();
    extract_text(path).with_context(|| format!("pdf-extract could not read {}", path.display()))
}
