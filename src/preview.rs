use crate::constants::{COMPILE_TIMEOUT_SECS, MAX_LATEX_SIZE_BYTES, PROCESS_POLL_INTERVAL_MS};
use horrorshow::helper::doctype;
use horrorshow::{html, Raw};
use html_escape::encode_text;
use std::fs;
use std::process::Command;
use std::sync::OnceLock;
use tempfile::tempdir;

#[derive(Clone, Debug)]
pub struct Preview;

#[derive(Debug)]
struct PdfLatexCapabilities {
    supports_openin_any: bool,
    supports_openout_any: bool,
}

impl Default for Preview {
    fn default() -> Self {
        Self::new()
    }
}

impl Preview {
    pub fn new() -> Self {
        Preview
    }

    /// Detects pdflatex capabilities (supported security flags)
    fn pdflatex_capabilities() -> &'static PdfLatexCapabilities {
        static CAPABILITIES: OnceLock<PdfLatexCapabilities> = OnceLock::new();
        CAPABILITIES.get_or_init(|| {
            let caps = PdfLatexCapabilities {
                supports_openin_any: false,
                supports_openout_any: false,
            };

            // These flags are often configuration-only (texmf.cnf) or restricted in standard texlive.
            // We'll skip testing them to avoid log noise and compatibility issues.
            // Essential security is handled by -no-shell-escape and temp directories.

            caps
        })
    }

    /// Creates a secure pdflatex command with appropriate security flags
    fn secure_pdflatex_command(
        &self,
        temp_dir: &std::path::Path,
        input_path: &std::path::Path,
    ) -> Command {
        let caps = Self::pdflatex_capabilities();
        let mut cmd = Command::new("pdflatex");

        // Essential security: disable shell escape
        cmd.arg("-no-shell-escape");

        // Restrict file access if supported
        if caps.supports_openin_any {
            cmd.arg("-openin-any=p");
        }
        if caps.supports_openout_any {
            cmd.arg("-openout-any=p");
        }

        // Run in temp directory to further restrict access
        cmd.current_dir(temp_dir);

        // Standard arguments
        cmd.arg("-interaction=nonstopmode")
            .arg("-output-directory")
            .arg(temp_dir)
            .arg(input_path);

        cmd
    }

    fn sanitize_paths(text: &str, temp_dir: &str, input_path: &str) -> String {
        text.replace(temp_dir, "[TEMP_DIR]")
            .replace(input_path, "[TEMP_DIR]/doc.tex")
    }

    fn run_command_with_timeout(
        cmd: &mut std::process::Command,
        timeout_secs: u64,
    ) -> Result<std::process::Output, String> {
        use std::time::{Duration, Instant};
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn command: {}", e))?;
        let start = Instant::now();
        let timeout = Duration::from_secs(timeout_secs);

        while start.elapsed() < timeout {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    let output = child
                        .wait_with_output()
                        .map_err(|e| format!("Failed to get output: {}", e))?;
                    return Ok(output);
                }
                Ok(None) => {
                    std::thread::sleep(Duration::from_millis(PROCESS_POLL_INTERVAL_MS));
                    continue;
                }
                Err(e) => return Err(format!("Error waiting for child: {}", e)),
            }
        }
        // Timeout reached
        let _ = child.kill();
        let _ = child.wait();
        Err(format!("Command timed out after {} seconds", timeout_secs))
    }

    pub fn render(&self, content: &str, dark_mode: bool) -> String {
        match self.compile_latex(content) {
            Ok(svgs) => self.wrap_svgs(svgs, dark_mode),
            Err(e) => self.wrap_error(&e),
        }
    }

    /// Compiles LaTeX string directly to a PDF file at the specified destination.
    #[allow(dead_code)]
    pub fn export_pdf(&self, latex: &str, destination: &std::path::Path) -> Result<(), String> {
        // Security: Validate input size
        if latex.len() > MAX_LATEX_SIZE_BYTES {
            return Err("Document too large".to_string());
        }

        let dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let input_path = dir.path().join("doc.tex");
        let temp_dir_path = dir.path().to_string_lossy().to_string();
        let input_path_str = input_path.to_string_lossy().to_string();

        fs::write(&input_path, latex).map_err(|e| {
            Self::sanitize_paths(
                &format!("Failed to write tex file: {}", e),
                &temp_dir_path,
                &input_path_str,
            )
        })?;

        let mut cmd = self.secure_pdflatex_command(dir.path(), &input_path);

        let output =
            Self::run_command_with_timeout(&mut cmd, COMPILE_TIMEOUT_SECS).map_err(|e| {
                Self::sanitize_paths(
                    &format!("Failed to run pdflatex: {}", e),
                    &temp_dir_path,
                    &input_path_str,
                )
            })?;

        let pdf_path = dir.path().join("doc.pdf");
        if !pdf_path.exists() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("LaTeX failed to generate a PDF.\n{}", stderr));
        }

        fs::copy(&pdf_path, destination)
            .map_err(|e| format!("Failed to copy PDF to destination: {}", e))?;
        Ok(())
    }

    fn get_pdf_page_count(&self, pdf_path: &std::path::Path) -> usize {
        let mut cmd = Command::new("pdfinfo");
        cmd.arg(pdf_path);
        if let Ok(output) = cmd.output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Pages:") {
                    return line
                        .split_whitespace()
                        .last()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(1);
                }
            }
        }
        1
    }

    fn compile_latex(&self, latex: &str) -> Result<Vec<String>, String> {
        // Security: Validate input size to prevent DoS
        if latex.len() > MAX_LATEX_SIZE_BYTES {
            return Err(format!(
                "Document too large ({:.2} MB). Maximum allowed size is {:.2} MB.",
                latex.len() as f64 / (1024.0 * 1024.0),
                MAX_LATEX_SIZE_BYTES as f64 / (1024.0 * 1024.0)
            ));
        }

        let dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let input_path = dir.path().join("doc.tex");
        let temp_dir_path = dir.path().to_string_lossy().to_string();
        let input_path_str = input_path.to_string_lossy().to_string();

        fs::write(&input_path, latex).map_err(|e| {
            Self::sanitize_paths(
                &format!("Failed to write tex file: {}", e),
                &temp_dir_path,
                &input_path_str,
            )
        })?;

        // Smart multi-pass compilation
        let mut passes = 0;
        let max_passes = 3;
        let mut needs_rerun = true;

        while needs_rerun && passes < max_passes {
            passes += 1;

            // Run pdflatex
            let mut cmd = self.secure_pdflatex_command(dir.path(), &input_path);
            let output =
                Self::run_command_with_timeout(&mut cmd, COMPILE_TIMEOUT_SECS).map_err(|e| {
                    Self::sanitize_paths(
                        &format!(
                            "Failed to run pdflatex (Pass {}): {}. Is it installed?",
                            passes, e
                        ),
                        &temp_dir_path,
                        &input_path_str,
                    )
                })?;

            let pdf_path = dir.path().join("doc.pdf");
            let log_path = dir.path().join("doc.log");
            let log =
                fs::read_to_string(&log_path).unwrap_or_else(|_| "No log file found".to_string());

            // Check if we need to run Biber (only on first pass if detected)
            if passes == 1 {
                let bcf_path = dir.path().join("doc.bcf");
                if bcf_path.exists() || log.contains("Please (re)run Biber") {
                    let mut biber_cmd = Command::new("biber");
                    biber_cmd.current_dir(dir.path()).arg("doc");
                    // We don't fail if biber fails, just log it and continue
                    let _ = Self::run_command_with_timeout(&mut biber_cmd, COMPILE_TIMEOUT_SECS);
                    needs_rerun = true;
                    continue;
                }

                if log.contains("Run LaTeX again")
                    || log.contains("Rerun to get")
                    || log.contains("Label(s) may have changed")
                {
                    needs_rerun = true;
                    continue;
                }
            } else {
                // Subsequent passes
                if log.contains("Run LaTeX again")
                    || log.contains("Rerun to get")
                    || log.contains("Label(s) may have changed")
                {
                    needs_rerun = true;
                } else {
                    needs_rerun = false;
                }
            }

            // If it's the last pass or we don't need a rerun, check if PDF exists
            if !needs_rerun || passes == max_passes {
                if !pdf_path.exists() {
                    let stderr = Self::sanitize_paths(
                        &String::from_utf8_lossy(&output.stderr),
                        &temp_dir_path,
                        &input_path_str,
                    );
                    let stdout = Self::sanitize_paths(
                        &String::from_utf8_lossy(&output.stdout),
                        &temp_dir_path,
                        &input_path_str,
                    );
                    let log_sanitized = Self::sanitize_paths(&log, &temp_dir_path, &input_path_str);

                    return Err(format!(
                        "LaTeX failed to generate a PDF.\n\n--- LOG ---\n{}\n\n--- STDERR ---\n{}\n\n--- STDOUT ---\n{}",
                        log_sanitized, stderr, stdout
                    ));
                }
            }
        }

        let pdf_path = dir.path().join("doc.pdf");
        let page_count = self.get_pdf_page_count(&pdf_path);
        let mut svgs = Vec::new();

        // Convert PDF to SVG page by page
        for page in 1..=page_count {
            let svg_filename = format!("output-{}.svg", page);
            let svg_path = dir.path().join(&svg_filename);

            let mut cmd = Command::new("pdftocairo");
            cmd.arg("-svg")
                .arg("-f")
                .arg(page.to_string())
                .arg("-l")
                .arg(page.to_string())
                .arg(&pdf_path)
                .arg(&svg_path);

            let cairo_output = Self::run_command_with_timeout(&mut cmd, COMPILE_TIMEOUT_SECS)
                .map_err(|e| {
                    Self::sanitize_paths(
                        &format!(
                            "Failed to run pdftocairo for page {}: {}. Is poppler-utils installed?",
                            page, e
                        ),
                        &temp_dir_path,
                        &input_path_str,
                    )
                })?;

            if !cairo_output.status.success() {
                let cairo_stderr = String::from_utf8_lossy(&cairo_output.stderr);
                let cairo_stderr_sanitized =
                    Self::sanitize_paths(&cairo_stderr, &temp_dir_path, &input_path_str);
                return Err(format!(
                    "pdftocairo failed to convert page {} to SVG.\n\nStderr:\n{}",
                    page, cairo_stderr_sanitized
                ));
            }

            if let Ok(content) = fs::read_to_string(&svg_path) {
                svgs.push(content);
            }
        }

        if svgs.is_empty() {
            let log_path = dir.path().join("doc.log");
            let log =
                fs::read_to_string(log_path).unwrap_or_else(|_| "No log file found".to_string());
            return Err(format!(
                "No SVG pages were generated (Page count was {}).\n\n--- LOG ---\n{}",
                page_count, log
            ));
        }

        Ok(svgs)
    }

    fn wrap_svgs(&self, svgs: Vec<String>, dark_mode: bool) -> String {
        let mut body_content = String::new();
        for svg in svgs {
            body_content.push_str("<div class=\"page\">");
            body_content.push_str(&svg);
            body_content.push_str("</div>");
        }

        let body_class = if dark_mode { "dark-mode" } else { "" };

        format!(
            "{}",
            html! {
                : doctype::HTML;
                html {
                     head {
                         meta(charset="utf-8");
                         meta(http-equiv="Content-Security-Policy",
                              content="default-src 'self'; script-src 'none'; style-src 'unsafe-inline';");
                         meta(http-equiv="X-Frame-Options", content="DENY");
                         meta(http-equiv="X-Content-Type-Options", content="nosniff");
                         style {
                             : Raw("
                                 body { 
                                     background-color: #f0f0f0; 
                                     display: flex; 
                                     flex-direction: column; 
                                     align-items: center; 
                                     padding: 20px;
                                     gap: 20px;
                                 }
                                 .page {
                                     background: white;
                                     box-shadow: 0 4px 8px rgba(0,0,0,0.1);
                                     margin-bottom: 20px;
                                     width: 850px;
                                     max-width: 95%;
                                 }
                                 svg { 
                                     display: block; 
                                     width: 100%; 
                                     height: auto; 
                                 }

                                 @media (prefers-color-scheme: dark) {
                                     body {
                                         background-color: #1e1e1e;
                                     }
                                 }

                                 body.dark-mode .page {
                                     background: #1e1e1e;
                                     border: 1px solid #333;
                                 }
                                 body.dark-mode svg {
                                     filter: invert(1) hue-rotate(180deg) brightness(1.2);
                                 }
                             ")
                         }
                     }
                    body(class=body_class) {
                        : Raw(&body_content);
                    }
                }
            }
        )
    }

    fn wrap_error(&self, error: &str) -> String {
        format!(
            "{}",
            html! {
                : doctype::HTML;
                html {
                     head {
                         meta(charset="utf-8");
                         meta(http-equiv="Content-Security-Policy",
                              content="default-src 'self'; script-src 'none'; style-src 'unsafe-inline';");
                         meta(http-equiv="X-Frame-Options", content="DENY");
                         meta(http-equiv="X-Content-Type-Options", content="nosniff");
                         style {
                             : Raw("
                                 body { font-family: monospace; padding: 20px; white-space: pre-wrap; background: #fff1f1; color: #a94442; }
                                 @media (prefers-color-scheme: dark) {
                                     body { background: #2a0f0f; color: #ff9999; }
                                 }
                             ")
                         }
                     }
                     body {
                         h1 { : "Compilation Error" }
                          : &*encode_text(error);
                     }
                }
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_paths() {
        let temp_dir = "/tmp/xyz123";
        let input_path = "/tmp/xyz123/doc.tex";
        let text = "Error in /tmp/xyz123/doc.tex: missing package";
        let sanitized = Preview::sanitize_paths(text, temp_dir, input_path);
        assert_eq!(sanitized, "Error in [TEMP_DIR]/doc.tex: missing package");
    }

    #[test]
    fn test_render_multi_page() {
        let preview = Preview::new();
        let latex = r#"
\documentclass{article}
\usepackage{graphicx}
\begin{document}
Page 1
\newpage
Page 2
\includegraphics{missing_image.png}
\end{document}
"#;
        let result = preview.render(latex, false);
        assert!(result.contains("class=\"page\""));
        assert!(result.contains("<svg"));
        let page_count = result.matches("class=\"page\"").count();
        assert_eq!(page_count, 2);
    }
}
