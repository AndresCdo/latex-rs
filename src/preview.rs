use horrorshow::helper::doctype;
use horrorshow::{html, Raw};
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

    pub fn render(&self, content: &str) -> String {
        match self.compile_latex(content) {
            Ok(svgs) => self.wrap_svgs(svgs),
            Err(e) => self.wrap_error(&e),
        }
    }

    fn compile_latex(&self, latex: &str) -> Result<Vec<String>, String> {
        let dir = tempdir().map_err(|e| format!("Failed to create temp dir: {}", e))?;
        let input_path = dir.path().join("doc.tex");
        fs::write(&input_path, latex).map_err(|e| format!("Failed to write tex file: {}", e))?;

        // Run pdflatex
        let output = Command::new("pdflatex")
            .arg("-interaction=nonstopmode")
            .arg("-output-directory")
            .arg(dir.path())
            .arg(&input_path)
            .output()
            .map_err(|e| format!("Failed to run pdflatex: {}. Is it installed?", e))?;

        let pdf_path = dir.path().join("doc.pdf");
        let log_path = dir.path().join("doc.log");
        let log = fs::read_to_string(&log_path).unwrap_or_else(|_| "No log file found".to_string());

        // If no PDF was generated, we MUST show the log or stderr to the user
        if !pdf_path.exists() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            return Err(format!(
                "LaTeX failed to generate a PDF.\n\n--- LOG ---\n{}\n\n--- STDERR ---\n{}\n\n--- STDOUT ---\n{}",
                log, stderr, stdout
            ));
        }

        // Convert PDF to SVG using pdftocairo
        // pdftocairo doc.pdf doc -svg -> doc-1.svg, doc-2.svg...
        let cairo_output = Command::new("pdftocairo")
            .arg("-svg")
            .arg(&pdf_path)
            .arg(dir.path().join("output"))
            .output()
            .map_err(|e| {
                format!(
                    "Failed to run pdftocairo: {}. Is poppler-utils installed?",
                    e
                )
            })?;

        let cairo_stderr = String::from_utf8_lossy(&cairo_output.stderr);
        if !cairo_output.status.success() {
            return Err(format!(
                "pdftocairo failed to convert PDF to SVG.\n\nStderr:\n{}",
                cairo_stderr
            ));
        }

        // Wait a small bit for files to be flushed to disk if needed (rare but possible on some FS)
        std::thread::sleep(std::time::Duration::from_millis(10));

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
        let paths =
            fs::read_dir(dir.path()).map_err(|e| format!("Failed to read temp dir: {}", e))?;
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
                .unwrap_or_else(|e| format!("Could not list directory: {}", e));

            return Err(format!(
                "No SVG pages were generated, but a PDF file exists.\n\
                Temp Dir Contents: [{}]\n\
                pdftocairo stderr: {}\n\
                Likely a LaTeX compilation error occurred.\n\n--- LOG ---\n{}",
                file_list, cairo_stderr, log
            ));
        }

        Ok(svgs)
    }

    fn wrap_svgs(&self, svgs: Vec<String>) -> String {
        let mut body_content = String::new();
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
                        style {
                            : Raw("
                                body { font-family: monospace; padding: 20px; white-space: pre-wrap; background: #fff1f1; color: #a94442; }
                            ")
                        }
                    }
                    body {
                        h1 { : "Compilation Error" }
                        : error;
                    }
                }
            }
        )
    }
}
