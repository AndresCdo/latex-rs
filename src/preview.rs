use crate::constants::{
    COMPILE_TIMEOUT_SECS, FS_FLUSH_DELAY_MS, MAX_LATEX_SIZE_BYTES, PROCESS_POLL_INTERVAL_MS,
};
use horrorshow::helper::doctype;
use horrorshow::{html, Raw};
use html_escape::encode_text;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[derive(Clone, Debug)]
pub struct Preview;

impl Default for Preview {
    fn default() -> Self {
        Self::new()
    }
}

impl Preview {
    pub fn new() -> Self {
        Preview
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

    pub fn render(&self, content: &str) -> String {
        match self.compile_latex(content) {
            Ok(svgs) => self.wrap_svgs(svgs),
            Err(e) => self.wrap_error(&e),
        }
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

        // Run pdflatex with security flags:
        // -no-shell-escape: Prevent command execution via \write18
        // Note: -openin-any=p removed as it is not supported by all pdflatex versions
        let mut cmd = Command::new("pdflatex");
        cmd.arg("-no-shell-escape")
            .arg("-interaction=nonstopmode")
            .arg("-output-directory")
            .arg(dir.path())
            .arg(&input_path);
        let output = Self::run_command_with_timeout(&mut cmd, COMPILE_TIMEOUT_SECS).map_err(|e| {
            Self::sanitize_paths(
                &format!("Failed to run pdflatex: {}. Is it installed?", e),
                &temp_dir_path,
                &input_path_str,
            )
        })?;

        let pdf_path = dir.path().join("doc.pdf");
        let log_path = dir.path().join("doc.log");
        let log = fs::read_to_string(&log_path).unwrap_or_else(|_| "No log file found".to_string());

        // If no PDF was generated, we MUST show the log or stderr to the user
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

        // Convert PDF to SVG using pdftocairo
        // pdftocairo doc.pdf doc -svg -> doc-1.svg, doc-2.svg...
        let mut cmd = Command::new("pdftocairo");
        cmd.arg("-svg")
            .arg(&pdf_path)
            .arg(dir.path().join("output"));
        let cairo_output = Self::run_command_with_timeout(&mut cmd, COMPILE_TIMEOUT_SECS).map_err(|e| {
            Self::sanitize_paths(
                &format!(
                    "Failed to run pdftocairo: {}. Is poppler-utils installed?",
                    e
                ),
                &temp_dir_path,
                &input_path_str,
            )
        })?;

        let cairo_stderr = String::from_utf8_lossy(&cairo_output.stderr);
        if !cairo_output.status.success() {
            let cairo_stderr_sanitized =
                Self::sanitize_paths(&cairo_stderr, &temp_dir_path, &input_path_str);
            return Err(format!(
                "pdftocairo failed to convert PDF to SVG.\n\nStderr:\n{}",
                cairo_stderr_sanitized
            ));
        }

        // Wait a small bit for files to be flushed to disk if needed (rare but possible on some FS)
        std::thread::sleep(std::time::Duration::from_millis(FS_FLUSH_DELAY_MS));

        let mut svgs = Vec::new();
        // Check if output.svg exists directly (for single page output)
        let direct_output = dir.path().join("output.svg");
        if direct_output.exists() {
            if let Ok(content) = fs::read_to_string(&direct_output) {
                svgs.push(content);
            }
        }

        // Check for "output" without extension (user environment quirk)
        let weird_output = dir.path().join("output");
        if weird_output.exists() && weird_output.is_file() {
            if let Ok(content) = fs::read_to_string(&weird_output) {
                // If it really is an SVG, push it.
                if content.contains("<svg") {
                    svgs.push(content);
                }
            }
        }

        // Look for output-*.svg files (multi-page only now)
        let paths = fs::read_dir(dir.path()).map_err(|e| {
            Self::sanitize_paths(
                &format!("Failed to read temp dir: {}", e),
                &temp_dir_path,
                &input_path_str,
            )
        })?;
        let mut svg_paths: Vec<_> = paths
            .filter_map(|p| p.ok())
            .filter(|p| {
                let name = p.file_name().to_string_lossy().to_string();
                name.starts_with("output-") && name.ends_with(".svg")
            })
            .collect();

        // Sort by page number (output-1.svg, output-2.svg...)
        svg_paths.sort_by_key(|p| {
            let name = p.file_name().to_string_lossy().to_string();
            let num: u32 = name
                .trim_start_matches("output-")
                .trim_end_matches(".svg")
                .parse()
                .unwrap_or(0);
            num
        });

        for entry in svg_paths {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                svgs.push(content);
            }
        }

        if svgs.is_empty() {
            let log_path = dir.path().join("doc.log");
            let log =
                fs::read_to_string(log_path).unwrap_or_else(|_| "No log file found".to_string());

            if log.contains("No pages of output") {
                return Err("Document compiled but generated no pages. Ensure you have content between \\begin{document} and \\end{document}.".to_string());
            }

            // List files in temp dir for debugging `pdftocairo` issues
            let file_list = fs::read_dir(dir.path())
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|e| {
                    Self::sanitize_paths(
                        &format!("Could not list directory: {}", e),
                        &temp_dir_path,
                        &input_path_str,
                    )
                });

            let cairo_stderr_sanitized =
                Self::sanitize_paths(&cairo_stderr, &temp_dir_path, &input_path_str);
            let log_sanitized = Self::sanitize_paths(&log, &temp_dir_path, &input_path_str);
            return Err(format!(
                "No SVG pages were generated, but a PDF file exists.\n\
                Temp Dir Contents: [{}]\n\
                pdftocairo stderr: {}\n\
                Likely a LaTeX compilation error occurred.\n\n--- LOG ---\n{}",
                file_list, cairo_stderr_sanitized, log_sanitized
            ));
        }

        Ok(svgs)
    }

    fn wrap_svgs(&self, svgs: Vec<String>) -> String {
        // Pre-calculate capacity to reduce allocations
        // Each page wrapper adds ~30 chars: <div class="page"></div>
        const PAGE_WRAPPER_OVERHEAD: usize = 30;
        let total_svg_size: usize = svgs.iter().map(|s| s.len()).sum();
        let estimated_capacity = total_svg_size + (svgs.len() * PAGE_WRAPPER_OVERHEAD);

        let mut body_content = String::with_capacity(estimated_capacity);
        for svg in svgs {
            body_content.push_str("<div class=\"page\">");
            body_content.push_str(&svg);
            body_content.push_str("</div>");
        }

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
                                     max-width: 100%;
                                 }
                                 svg { display: block; width: 100%; height: auto; }

                                 @media (prefers-color-scheme: dark) {
                                     body {
                                         background-color: #1e1e1e;
                                     }
                                 }
                             ")
                         }
                     }
                    body {
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

        let text2 = "Paths: /tmp/xyz123 and /tmp/xyz123/doc.tex";
        let sanitized2 = Preview::sanitize_paths(text2, temp_dir, input_path);
        assert_eq!(sanitized2, "Paths: [TEMP_DIR] and [TEMP_DIR]/doc.tex");

        let text3 = "No paths here";
        let sanitized3 = Preview::sanitize_paths(text3, temp_dir, input_path);
        assert_eq!(sanitized3, "No paths here");
    }
}
