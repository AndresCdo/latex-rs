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

pub fn extract_sections(text: &str) -> Vec<(String, i32)> {
    let mut sections = Vec::new();
    let re = regex::Regex::new(r"\\(section|subsection|subsubsection)\*?\{([^}]+)\}").unwrap();
    
    for (i, line) in text.lines().enumerate() {
        if let Some(caps) = re.captures(line) {
            let level = &caps[1];
            let title = &caps[2];
            let prefix = match level {
                "section" => "",
                "subsection" => "  ",
                "subsubsection" => "    ",
                _ => "",
            };
            sections.push((format!("{}{}", prefix, title), i as i32));
        }
    }
    sections
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
    let raw = if let Some(start_idx) = response.find("```latex") {
        let after_start = &response[start_idx + 8..];
        if let Some(end_idx) = after_start.find("```") {
            after_start[..end_idx].trim().to_string()
        } else {
            after_start.trim().to_string()
        }
    } else if let Some(start_idx) = response.find("```") {
        let after_start = &response[start_idx + 3..];
        if let Some(end_idx) = after_start.find("```") {
            after_start[..end_idx].trim().to_string()
        } else {
            after_start.trim().to_string()
        }
    } else if let Some(start_idx) = response.find("\\documentclass") {
        if let Some(end_idx) = response.rfind("\\end{document}") {
            response[start_idx..end_idx + 14].to_string()
        } else {
            response[start_idx..].trim().to_string()
        }
    } else {
        response.trim().to_string()
    };

    sanitize_latex(&raw)
}

/// Post-processes AI-generated LaTeX to fix common hallucinations and formatting issues.
fn sanitize_latex(latex: &str) -> String {
    // 1. Normalize line endings (Remove \r)
    let mut sanitized = latex.replace("\r\n", "\n").replace('\r', "\n");

    // 2. Fix common AI document class hallucinations
    // Small models like qwen often put package names in \documentclass
    if sanitized.contains("\\documentclass{amsmath}")
        || sanitized.contains("\\documentclass{amssymb}")
        || sanitized.contains("\\documentclass{geometry}")
    {
        tracing::warn!("AI hallucinated document class; correcting to 'article'");
        sanitized = sanitized
            .replace("\\documentclass{amsmath}", "\\documentclass{article}\n\\usepackage{amsmath}")
            .replace("\\documentclass{amssymb}", "\\documentclass{article}\n\\usepackage{amssymb}")
            .replace("\\documentclass{geometry}", "\\documentclass{article}\n\\usepackage{geometry}");
    }

    // 3. Ensure it starts with \documentclass if it looks like a full document
    if sanitized.contains("\\begin{document}") && !sanitized.contains("\\documentclass") {
        sanitized = format!("\\documentclass{{article}}\n\\usepackage{{amsmath}}\n\\usepackage{{amssymb}}\n\n{}", sanitized);
    }

    sanitized.trim().to_string()
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

    #[test]
    fn test_open_file_not_found() {
        let path = std::env::temp_dir().join("nonexistent_file_12345.txt");
        let result = open_file(&path);
        assert!(result.is_err());
    }

    // =========================================================================
    // Tests for extract_latex
    // =========================================================================

    #[test]
    fn test_extract_latex_from_latex_block() {
        let response = r#"Here is your document:
```latex
\documentclass{article}
\begin{document}
Hello World
\end{document}
```
Hope this helps!"#;

        let result = extract_latex(response);
        assert!(result.contains("\\documentclass{article}"));
        assert!(result.contains("\\end{document}"));
        assert!(!result.contains("Hope this helps"));
    }

    #[test]
    fn test_extract_latex_from_generic_block() {
        let response = r#"```
\documentclass{article}
\begin{document}
Test
\end{document}
```"#;

        let result = extract_latex(response);
        assert!(result.contains("\\documentclass{article}"));
    }

    #[test]
    fn test_extract_latex_from_raw_document() {
        let response = r#"Sure! Here is the document:
\documentclass{article}
\begin{document}
Content here
\end{document}
Let me know if you need changes."#;

        let result = extract_latex(response);
        assert!(result.starts_with("\\documentclass"));
        assert!(result.ends_with("\\end{document}"));
    }

    #[test]
    fn test_extract_latex_partial_document() {
        let response = "\\documentclass{article}\n\\usepackage{amsmath}";

        let result = extract_latex(response);
        assert!(result.contains("\\documentclass{article}"));
        assert!(result.contains("\\usepackage{amsmath}"));
    }

    #[test]
    fn test_extract_latex_plain_text() {
        let response = "This is just plain text without any LaTeX";
        let result = extract_latex(response);
        assert_eq!(result, "This is just plain text without any LaTeX");
    }

    // =========================================================================
    // Tests for clean_diff (via apply_patch behavior)
    // =========================================================================

    #[test]
    fn test_apply_patch_simple() {
        let original = "line1\nline2\nline3\n";
        let patch = r#"--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 line1
-line2
+modified line2
 line3
"#;

        let result = apply_patch(original, patch);
        assert!(result.is_ok());
        let patched = result.unwrap();
        assert!(patched.contains("modified line2"));
        assert!(!patched.contains("\nline2\n"));
    }

    #[test]
    fn test_apply_patch_with_markdown_wrapper() {
        let original = "hello\nworld\n";
        let patch = r#"Here is the diff:
```diff
--- a/file.txt
+++ b/file.txt
@@ -1,2 +1,2 @@
-hello
+goodbye
 world
```
Done!"#;

        let result = apply_patch(original, patch);
        assert!(result.is_ok());
        let patched = result.unwrap();
        assert!(patched.contains("goodbye"));
    }

    #[test]
    fn test_apply_patch_invalid_format() {
        let original = "some content";
        let patch = "This is not a valid patch at all";

        let result = apply_patch(original, patch);
        // When the patch doesn't contain diff markers, clean_diff returns the original string,
        // which then fails to parse as a valid patch or produces unexpected results.
        // The behavior depends on diffy's handling - it may return Ok with unchanged content
        // or Err. We just verify it doesn't panic and handles gracefully.
        match result {
            Ok(patched) => {
                // If it "succeeds", the content should be unchanged since no valid patch was applied
                assert_eq!(patched.trim(), original.trim());
            }
            Err(_) => {
                // Expected - invalid patch format
            }
        }
    }

    #[test]
    fn test_apply_patch_context_mismatch() {
        let original = "completely different content\n";
        let patch = r#"--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,3 @@
 line1
-line2
+changed
 line3
"#;

        let result = apply_patch(original, patch);
        // Should fail because context doesn't match
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_patch_addition_only() {
        let original = "line1\nline3\n";
        let patch = r#"--- a/file.txt
+++ b/file.txt
@@ -1,2 +1,3 @@
 line1
+line2
 line3
"#;

        let result = apply_patch(original, patch);
        assert!(result.is_ok());
        let patched = result.unwrap();
        assert!(patched.contains("line2"));
    }

    #[test]
    fn test_apply_patch_deletion_only() {
        let original = "line1\nline2\nline3\n";
        let patch = r#"--- a/file.txt
+++ b/file.txt
@@ -1,3 +1,2 @@
 line1
-line2
 line3
"#;

        let result = apply_patch(original, patch);
        assert!(result.is_ok());
        let patched = result.unwrap();
        assert!(!patched.contains("line2"));
    }

    // =========================================================================
    // Tests for buffer_to_string (indirect, as it requires GTK)
    // =========================================================================
    // Note: buffer_to_string requires GTK initialization which is not
    // available in unit tests. Integration tests would be needed for this.
}
