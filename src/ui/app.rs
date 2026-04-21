use crate::util::processer::Processor;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Spectrix {
    sessions: Vec<Processor>,
    current_session: usize,
}

impl Default for Spectrix {
    fn default() -> Self {
        Self {
            sessions: vec![Processor::new(Self::default_session_name(1))],
            current_session: 0,
        }
    }
}

impl Spectrix {
    const SCREENSHOT_FILE_NAME: &str = "spectrix-screenshot.png";

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            let loaded: Option<Self> = eframe::get_value(storage, eframe::APP_KEY);
            if loaded.is_none() && storage.get_string(eframe::APP_KEY).is_some() {
                log::error!("Failed to restore persisted app state; falling back to defaults");
            }
            let mut app = loaded.unwrap_or_default();
            app.normalize_sessions();
            app
        } else {
            Default::default()
        }
    }

    pub fn reset_to_default(&mut self) {
        *self = Default::default();
    }

    fn default_session_name(index: usize) -> String {
        format!("Session {index}")
    }

    fn session_name_from_processor(processor: &Processor, index: usize) -> String {
        let fallback = Self::default_session_name(index + 1);
        let name = processor.settings.name.trim();
        if name.is_empty() {
            fallback
        } else {
            name.to_owned()
        }
    }

    fn next_default_session_name(&self) -> String {
        let used_numbers = self
            .sessions
            .iter()
            .filter_map(|session| {
                session
                    .settings
                    .name
                    .trim()
                    .strip_prefix("Session ")
                    .and_then(|suffix| suffix.parse::<usize>().ok())
            })
            .collect::<std::collections::BTreeSet<_>>();

        let next_number = (1..)
            .find(|number| !used_numbers.contains(number))
            .unwrap_or(1);

        Self::default_session_name(next_number)
    }

    fn add_session(&mut self) {
        let name = self.next_default_session_name();
        self.sessions.push(Processor::new(name));
        self.current_session = self.sessions.len() - 1;
    }

    fn remove_session(&mut self, index: usize) {
        if self.sessions.len() <= 1 || index >= self.sessions.len() {
            return;
        }

        self.sessions.remove(index);

        if self.current_session > index {
            self.current_session -= 1;
        } else if self.current_session >= self.sessions.len() {
            self.current_session = self.sessions.len() - 1;
        }
    }

    fn reset_session(&mut self, index: usize) {
        if let Some(session) = self.sessions.get_mut(index) {
            session.reset();
        }
    }

    fn move_session(&mut self, from: usize, insertion_index: usize) {
        let len = self.sessions.len();
        if from >= len || insertion_index > len {
            return;
        }

        let target = if from < insertion_index {
            insertion_index - 1
        } else {
            insertion_index
        };

        if from == target {
            return;
        }

        let session = self.sessions.remove(from);
        self.sessions.insert(target, session);

        self.current_session = if self.current_session == from {
            target
        } else {
            let current_after_removal = if self.current_session > from {
                self.current_session - 1
            } else {
                self.current_session
            };

            if current_after_removal >= target {
                current_after_removal + 1
            } else {
                current_after_removal
            }
        };
    }

    fn normalize_sessions(&mut self) {
        if self.sessions.is_empty() {
            self.sessions
                .push(Processor::new(Self::default_session_name(1)));
            self.current_session = 0;
        }

        for (index, session) in self.sessions.iter_mut().enumerate() {
            if session.settings.name.trim().is_empty() {
                session.settings.name = Self::default_session_name(index + 1);
            }
        }

        if self.current_session >= self.sessions.len() {
            self.current_session = self.sessions.len() - 1;
        }
    }

    fn ensure_extension_if_missing(path: PathBuf, extension: &str) -> PathBuf {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case(extension) => path,
            _ => path.with_extension(extension),
        }
    }

    fn save_screenshot_path_dialog() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .set_file_name(Self::SCREENSHOT_FILE_NAME)
            .save_file()
            .map(|path| Self::ensure_extension_if_missing(path, "png"))
    }

    fn request_screenshot(ctx: &egui::Context, path: PathBuf) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(path)));
        ctx.request_repaint();
    }

    fn save_color_image_png(path: &Path, image: &egui::ColorImage) -> Result<(), String> {
        let [width, height] = image.size;
        let rgba: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_srgba_unmultiplied())
            .collect();

        image::save_buffer(
            path,
            &rgba,
            width as u32,
            height as u32,
            image::ColorType::Rgba8,
        )
        .map_err(|err| err.to_string())
    }

    fn handle_screenshot_events(ctx: &egui::Context) {
        let screenshot_events = ctx.input(|input| {
            input
                .events
                .iter()
                .filter_map(|event| match event {
                    egui::Event::Screenshot {
                        viewport_id,
                        user_data,
                        image,
                    } if *viewport_id == egui::ViewportId::ROOT => user_data
                        .data
                        .as_ref()
                        .and_then(|data| data.downcast_ref::<PathBuf>())
                        .map(|path| (path.clone(), image.clone())),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });

        for (path, image) in screenshot_events {
            match Self::save_color_image_png(&path, image.as_ref()) {
                Ok(()) => {
                    log::info!("Screenshot saved to {}", path.display());
                }
                Err(err) => {
                    log::error!("Failed to save screenshot to {}: {}", path.display(), err);
                }
            }
        }
    }
}

impl eframe::App for Spectrix {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        Self::handle_screenshot_events(ui.ctx());

        egui::Panel::top("spectrix_top_panel").show_inside(ui, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("spectrix_top_scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            egui::global_theme_preference_switch(ui);
                            ui.heading("Spectrix");
                            ui.separator();

                            if ui
                                .button("\u{2399}")
                                .on_hover_text("Capture the current window to a PNG file")
                                .clicked()
                                && let Some(path) = Self::save_screenshot_path_dialog()
                            {
                                Self::request_screenshot(ui.ctx(), path);
                            }
                            ui.separator();

                            let can_remove_session = self.sessions.len() > 1;
                            let mut pending_reset = None;
                            let mut pending_remove = None;
                            let mut pending_move = None;
                            let session_count = self.sessions.len();

                            for i in 0..session_count {
                                let fallback_name = Self::default_session_name(i + 1);
                                let label = Self::session_name_from_processor(&self.sessions[i], i);
                                let response = ui
                                    .add(
                                        egui::Button::selectable(self.current_session == i, label)
                                            .sense(egui::Sense::click_and_drag()),
                                    )
                                    .on_hover_text(
                                        "Click to switch. Drag to reorder. Right click for session options",
                                    );
                                let selected = response.clicked() || response.secondary_clicked();
                                let tab_rect = response.rect;
                                let session = &mut self.sessions[i];

                                response.dnd_set_drag_payload(i);

                                egui::Popup::context_menu(&response)
                                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                                    .show(|ui| {
                                        ui.set_min_width(220.0);
                                        ui.label("Session Name");

                                        let name_response =
                                            ui.text_edit_singleline(&mut session.settings.name);

                                        if name_response.lost_focus()
                                            && session.settings.name.trim().is_empty()
                                        {
                                            session.settings.name = fallback_name.clone();
                                        }

                                        ui.separator();

                                        session.session_processor_menu_ui(ui);

                                        ui.separator();

                                        ui.horizontal(|ui| {
                                            if ui.button("Reset").clicked() {
                                                session.histogrammer = Default::default();
                                                ui.close();
                                            }

                                            ui.menu_button("Histogrammer", |ui| {
                                                session.histogrammer.menu_contents_ui(ui);
                                            });
                                        });

                                        ui.separator();

                                        ui.horizontal( |ui| {
                                            ui.label("Session:");
                                                if ui.button("Reset").clicked() {
                                                    if session.settings.name.trim().is_empty() {
                                                        session.settings.name = fallback_name.clone();
                                                    }
                                                pending_reset = Some(i);
                                                ui.close();
                                            }

                                            if ui
                                                .add_enabled(
                                                    can_remove_session,
                                                    egui::Button::new("Remove"),
                                                )
                                                .clicked()
                                            {
                                                pending_remove = Some(i);
                                                ui.close();
                                            }
                                        });
                                    });

                                if let Some(dragged_index) = response.dnd_release_payload::<usize>() {
                                    let insert_index =
                                        ui.pointer_interact_pos().map_or(i, |pos| {
                                            if pos.x >= tab_rect.center().x {
                                                i + 1
                                            } else {
                                                i
                                            }
                                        });
                                    pending_move = Some((*dragged_index, insert_index));
                                }

                                if selected {
                                    self.current_session = i;
                                }

                                ui.separator();
                            }

                            if ui.button("➕ New Session").clicked() {
                                self.add_session();
                            }

                            if let Some(index) = pending_remove {
                                self.remove_session(index);
                            } else if let Some(index) = pending_reset {
                                self.reset_session(index);
                            } else if let Some((from, to)) = pending_move {
                                self.move_session(from, to);
                            }
                        });
                    });
            });
        });

        // Draw the UI for the current session
        if let Some(current) = self.sessions.get_mut(self.current_session) {
            current.ui(ui);
        }
    }
}
