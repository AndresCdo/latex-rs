use anyhow::{Context, Result};
use gtk4::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::Path;

pub fn buffer_to_string(buffer: &gtk4::TextBuffer) -> String {
    let (start, end) = buffer.bounds();
    buffer.text(&start, &end, false).to_string()
}

pub fn check_dependencies() -> Vec<String> {
    let mut missing = Vec::new();

    // Check pdflatex
    if std::process::Command::new("pdflatex")
        .arg("--version")
        .output()
        .is_err()
    {
        missing.push("pdflatex (texlive-latex-base)".to_string());
    }

    // Check pdftocairo
    if std::process::Command::new("pdftocairo")
        .arg("-v")
        .output()
        .is_err()
    {
        missing.push("pdftocairo (poppler-utils)".to_string());
    }

    missing
}

pub fn open_file(filename: &Path) -> Result<String> {
    let file =
        File::open(filename).with_context(|| format!("Failed to open file: {:?}", filename))?;
    let mut reader = BufReader::new(file);
    let mut contents = String::new();
    reader
        .read_to_string(&mut contents)
        .with_context(|| "Failed to read file contents")?;
    Ok(contents)
}

pub fn save_file(filename: &Path, text_buffer: &gtk4::TextBuffer) -> Result<()> {
    let contents = buffer_to_string(text_buffer);
    let temp_filename = filename.with_extension("tmp");

    // Write contents to a temporary file first for atomic-like saving
    let mut file = File::create(&temp_filename)
        .with_context(|| format!("Failed to create temporary file: {:?}", temp_filename))?;

    file.write_all(contents.as_bytes())
        .with_context(|| "Failed to write content to temporary file")?;

    file.sync_all()
        .with_context(|| "Failed to sync temporary file")?;

    // Atomic rename
    std::fs::rename(&temp_filename, filename)
        .with_context(|| format!("Failed to rename temporary file to {:?}", filename))?;

    Ok(())
}

pub fn apply_patch(current: &str, patch: &str) -> Result<String> {
    let cleaned_patch = clean_diff(patch);
    tracing::debug!(
        "AI Assistant: Cleaned patch used for applying: {}",
        cleaned_patch
    );

    let patch_obj = diffy::Patch::from_str(&cleaned_patch)
        .map_err(|e| anyhow::anyhow!("Invalid patch format: {}", e))?;

    diffy::apply(current, &patch_obj).map_err(|e| anyhow::anyhow!("Failed to apply patch: {}", e))
}

fn clean_diff(patch: &str) -> String {
    // LLMs often wrap the diff in markdown code blocks.
    // We want to extract just the diff part starting from "---" or "@@"

    let mut cleaned = String::new();
    let mut found_start = false;

    for line in patch.lines() {
        if !found_start {
            if line.starts_with("---") || line.starts_with("@@") {
                found_start = true;
                cleaned.push_str(line);
                cleaned.push('\n');
            }
            continue;
        }

        // Stop if we see ending backticks of a code block or unrelated text
        if line.starts_with("```") {
            break;
        }

        cleaned.push_str(line);
        cleaned.push('\n');
    }

    if !found_start {
        return patch.to_string();
    }

    cleaned
}

pub fn extract_latex(response: &str) -> String {
    // If there is a latex code block, take its content
    if let Some(start_idx) = response.find("```latex") {
        let after_start = &response[start_idx + 8..];
        if let Some(end_idx) = after_start.find("```") {
            return after_start[..end_idx].trim().to_string();
        }
    }

    // Fallback: generic code block
    if let Some(start_idx) = response.find("```") {
        let after_start = &response[start_idx + 3..];
        if let Some(end_idx) = after_start.find("```") {
            return after_start[..end_idx].trim().to_string();
        }
    }

    // Heuristic: If it looks like a full document, find \documentclass and \end{document}
    if let Some(start_idx) = response.find("\\documentclass") {
        if let Some(end_idx) = response.rfind("\\end{document}") {
            return response[start_idx..end_idx + 14].to_string();
        }
        // If no end found, or it's a snippet, just take from \documentclass onwards
        return response[start_idx..].trim().to_string();
    }

    response.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_open_file() {
        let path = std::env::temp_dir().join("test_latex_rs.txt");
        fs::write(&path, "Hello LaTeX").unwrap();

        let content = open_file(&path).unwrap();
        assert_eq!(content, "Hello LaTeX");

        fs::remove_file(path).unwrap();
    }
}
