use crate::constants::MAX_LATEX_SIZE_BYTES;
use crate::utils::{open_file, save_file};
use crate::AppState;
use adw::{ApplicationWindow, ToastOverlay};
use glib;
use gtk4::gio::prelude::FileExt;
use gtk4::prelude::{ButtonExt, Cast, TextBufferExt};
use gtk4::Button;
use sourceview5::Buffer;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

/// Connects the export button to the PDF generation logic using `pdflatex`.
pub fn connect_export_pdf(
    export_btn: &Button,
    window: &ApplicationWindow,
    buffer: &Buffer,
    _state: Rc<RefCell<AppState>>,
    toast_overlay: &ToastOverlay,
) {
    export_btn.connect_clicked(glib::clone!(
        #[weak]
        window,
        #[weak]
        buffer,
        #[weak]
        toast_overlay,
        move |_| {
            let text = crate::utils::buffer_to_string(buffer.upcast_ref());
            if text.len() > MAX_LATEX_SIZE_BYTES {
                toast_overlay.add_toast(adw::Toast::new(
                    "Document too large for PDF export (max 10 MB).",
                ));
                return;
            }

            let file_dialog = gtk4::FileDialog::builder()
                .title("Export PDF")
                .accept_label("Export")
                .modal(true)
                .build();

            file_dialog.save(
                Some(&window),
                None::<&gtk4::gio::Cancellable>,
                glib::clone!(
                    #[weak]
                    buffer,
                    #[weak]
                    toast_overlay,
                    move |result| {
                        match result {
                            Ok(gfile) => {
                                let path = gfile.path().expect("No path returned");
                                let _path_str = path.to_string_lossy();

                                // Ensure .pdf extension
                                let mut path_buf = path.to_path_buf();
                                if path_buf.extension().is_none_or(|ext| ext != "pdf") {
                                    path_buf.set_extension("pdf");
                                }

                                // Save temporary .tex file
                                let temp_dir = std::env::temp_dir();
                                let temp_tex = temp_dir.join("export_temp.tex");
                                if let Err(e) = std::fs::write(
                                    &temp_tex,
                                    crate::utils::buffer_to_string(buffer.upcast_ref()),
                                ) {
                                    toast_overlay.add_toast(adw::Toast::new(&format!(
                                        "Failed to create temp file: {}",
                                        e
                                    )));
                                    return;
                                }

                                // Run pdflatex
                                let output = Command::new("pdflatex")
                                    .arg("-interaction=nonstopmode")
                                    .arg("-output-directory")
                                    .arg(&temp_dir)
                                    .arg(&temp_tex)
                                    .output();

                                match output {
                                    Ok(output) if output.status.success() => {
                                        let pdf_path = temp_dir.join("export_temp.pdf");
                                        if pdf_path.exists() {
                                            if let Err(e) = std::fs::copy(&pdf_path, &path_buf) {
                                                toast_overlay.add_toast(adw::Toast::new(&format!(
                                                    "Failed to copy PDF: {}",
                                                    e
                                                )));
                                            } else {
                                                toast_overlay.add_toast(adw::Toast::new(&format!(
                                                    "PDF exported to {}",
                                                    path_buf.display()
                                                )));
                                            }
                                            // Cleanup
                                            let _ = std::fs::remove_file(&temp_tex);
                                            let _ = std::fs::remove_file(&pdf_path);
                                            let _ = std::fs::remove_file(
                                                temp_dir.join("export_temp.aux"),
                                            );
                                            let _ = std::fs::remove_file(
                                                temp_dir.join("export_temp.log"),
                                            );
                                        } else {
                                            toast_overlay.add_toast(adw::Toast::new(
                                                "PDF generation failed (no output file).",
                                            ));
                                        }
                                    }
                                    Ok(output) => {
                                        let stderr = String::from_utf8_lossy(&output.stderr);
                                        toast_overlay.add_toast(adw::Toast::new(&format!(
                                            "PDF compilation failed: {}",
                                            stderr.lines().next().unwrap_or("Unknown error")
                                        )));
                                    }
                                    Err(e) => {
                                        toast_overlay.add_toast(adw::Toast::new(&format!(
                                            "Failed to run pdflatex: {}",
                                            e
                                        )));
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("File dialog cancelled: {}", e);
                            }
                        }
                    }
                ),
            );
        }
    ));
}
#[allow(clippy::too_many_arguments)]
/// Connects standard file operations (New, Open, Save) and status bar updates
/// (cursor position, word count).
pub fn connect_file_operations(
    new_btn: &gtk4::Button,
    open_btn: &gtk4::Button,
    save_btn: &gtk4::Button,
    window: &adw::ApplicationWindow,
    buffer: &sourceview5::Buffer,
    state: Rc<RefCell<AppState>>,
    view_title: &adw::WindowTitle,
    pos_label: &gtk4::Label,
    word_count_label: &gtk4::Label,
) {
    // New button
    new_btn.connect_clicked(glib::clone!(
        #[weak]
        buffer,
        #[strong]
        state,
        #[weak]
        view_title,
        move |_| {
            buffer.set_text("");
            state.borrow_mut().current_file = None;
            view_title.set_subtitle("");
        }
    ));

    // Status bar updates
    buffer.connect_cursor_position_notify(glib::clone!(
        #[weak]
        pos_label,
        move |buf| {
            let buf = buf.upcast_ref::<gtk4::TextBuffer>();
            let iter = if let Some(cursor_mark) = buf.mark("insert") {
                buf.iter_at_mark(&cursor_mark)
            } else {
                buf.start_iter()
            };
            let line = iter.line() + 1;
            let col = iter.line_offset() + 1;
            pos_label.set_text(&format!("Line: {}, Col: {}", line, col));
        }
    ));

    buffer.connect_changed(glib::clone!(
        #[weak]
        word_count_label,
        move |buf| {
            let text = crate::utils::buffer_to_string(buf.upcast_ref());
            let words = text.split_whitespace().count();
            word_count_label.set_text(&format!("Words: {}", words));
        }
    ));

    // Open button
    open_btn.connect_clicked(glib::clone!(
        #[weak]
        window,
        #[weak]
        buffer,
        #[strong]
        state,
        #[weak]
        view_title,
        move |_| {
            let dialog = gtk4::FileDialog::builder().title("Open File").build();

            dialog.open(
                Some(&window),
                None::<&gio::Cancellable>,
                glib::clone!(
                    #[strong]
                    state,
                    #[weak]
                    buffer,
                    #[weak]
                    view_title,
                    move |res| {
                        if let Ok(file) = res {
                            if let Some(path) = file.path() {
                                if let Ok(content) = open_file(&path) {
                                    buffer.set_text(&content);
                                    state.borrow_mut().current_file = Some(path.to_path_buf());
                                    view_title.set_subtitle(&path.to_string_lossy());
                                }
                            }
                        }
                    }
                ),
            );
        }
    ));

    // Save button
    save_btn.connect_clicked(glib::clone!(
        #[weak]
        window,
        #[weak]
        buffer,
        #[strong]
        state,
        #[weak]
        view_title,
        move |_| {
            let path_opt = state.borrow().current_file.clone();
            if let Some(path) = path_opt {
                if let Err(e) = save_file(&path, buffer.upcast_ref()) {
                    tracing::error!("Failed to save: {}", e);
                }
            } else {
                let dialog = gtk4::FileDialog::builder().title("Save File").build();

                dialog.save(
                    Some(&window),
                    None::<&gio::Cancellable>,
                    glib::clone!(
                        #[strong]
                        state,
                        #[weak]
                        buffer,
                        #[weak]
                        view_title,
                        move |res| {
                            if let Ok(file) = res {
                                if let Some(path) = file.path() {
                                    if save_file(&path, buffer.upcast_ref()).is_ok() {
                                        state.borrow_mut().current_file = Some(path.to_path_buf());
                                        view_title.set_subtitle(&path.to_string_lossy());
                                    }
                                }
                            }
                        }
                    ),
                );
            }
        }
    ));
}
