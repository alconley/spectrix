use crate::defaults::{DEFAULTS_SCHEMA_VERSION, SpectrixDefaults};
use crate::util::processer::Processor;
use std::path::{Path, PathBuf};

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Spectrix {
    sessions: Vec<Processor>,
    current_session: usize,
    #[serde(default)]
    defaults: SpectrixDefaults,
    #[serde(default)]
    defaults_schema_version: u32,
    #[serde(default)]
    defaults_panel_open: bool,
    #[serde(skip)]
    defaults_search: String,
}

impl Default for Spectrix {
    fn default() -> Self {
        let defaults = SpectrixDefaults::default();
        Self {
            sessions: vec![Processor::new_with_defaults(
                Self::default_session_name(1),
                &defaults.new_sessions,
            )],
            current_session: 0,
            defaults,
            defaults_schema_version: DEFAULTS_SCHEMA_VERSION,
            defaults_panel_open: false,
            defaults_search: String::new(),
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
            app.migrate_defaults_if_needed();
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
        self.sessions.push(Processor::new_with_defaults(
            name,
            &self.defaults.new_sessions,
        ));
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
            session.reset_with_defaults(&self.defaults.new_sessions);
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
            self.sessions.push(Processor::new_with_defaults(
                Self::default_session_name(1),
                &self.defaults.new_sessions,
            ));
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

    fn migrate_defaults_if_needed(&mut self) {
        if self.defaults_schema_version >= DEFAULTS_SCHEMA_VERSION {
            return;
        }

        if let Some(session) = self.sessions.get(self.current_session) {
            self.defaults.general.estimated_memory_gb = session.settings.estimated_memory.max(0.1);
            self.defaults.ai = session.ai.app_defaults();
            self.defaults.new_sessions.files_panel_open = session.settings.left_panel_open;
            self.defaults.new_sessions.histogram_script_open =
                session.settings.histogram_script_open;
            self.defaults.new_sessions.ai_open = session.settings.ai_open;
            self.defaults.new_sessions.calculate_histograms_separately =
                session.settings.calculate_histograms_seperately;
            self.defaults.new_sessions.file_sort = session.file_sort;
            self.defaults.histogrammer = session.histogrammer.behavior.layout_defaults();
        }

        self.defaults_schema_version = DEFAULTS_SCHEMA_VERSION;
    }

    fn defaults_panel_ui(&mut self, ui: &mut egui::Ui) {
        let mut open = self.defaults_panel_open;
        egui::Panel::left("spectrix_defaults_panel")
            .resizable(true)
            .default_size(320.0)
            .size_range(260.0..=520.0)
            .show_collapsible(ui, &mut open, |ui| {
                super::defaults_panel::show(ui, &mut self.defaults, &mut self.defaults_search);
            });
        self.defaults_panel_open = open;
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
        for session in &mut self.sessions {
            session.histogrammer.set_app_defaults(&self.defaults);
        }

        egui::Panel::top("spectrix_top_panel").show(ui, |ui| {
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
                                .selectable_label(self.defaults_panel_open, "Defaults")
                                .on_hover_text("Open app-wide defaults")
                                .clicked()
                            {
                                self.defaults_panel_open = !self.defaults_panel_open;
                            }
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

                                        session.session_processor_menu_ui(ui, &mut self.defaults);

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

        self.defaults_panel_ui(ui);

        // Draw the UI for the current session
        if let Some(current) = self.sessions.get_mut(self.current_session) {
            current.ui(ui, &mut self.defaults);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Spectrix;
    use crate::defaults::DEFAULTS_SCHEMA_VERSION;
    use crate::histoer::pane::Pane;

    #[test]
    fn legacy_session_preferences_migrate_to_app_defaults() {
        let mut app = Spectrix::default();
        app.defaults_schema_version = 0;
        app.sessions[0].settings.estimated_memory = 2.5;
        app.sessions[0].settings.left_panel_open = false;
        app.sessions[0].settings.calculate_histograms_seperately = true;
        app.migrate_defaults_if_needed();

        assert_eq!(app.defaults_schema_version, DEFAULTS_SCHEMA_VERSION);
        assert_eq!(app.defaults.general.estimated_memory_gb, 2.5);
        assert!(!app.defaults.new_sessions.files_panel_open);
        assert!(app.defaults.new_sessions.calculate_histograms_separately);
        assert_eq!(app.sessions.len(), 1);
    }

    #[test]
    fn legacy_app_without_defaults_fields_deserializes() {
        let mut app: Spectrix =
            ron::from_str("(sessions:[],current_session:0)").expect("legacy app state");
        app.normalize_sessions();
        app.migrate_defaults_if_needed();

        assert_eq!(app.defaults_schema_version, DEFAULTS_SCHEMA_VERSION);
        assert_eq!(app.sessions.len(), 1);
        assert!(!app.defaults_panel_open);
    }

    #[test]
    fn workspace_with_histogram_and_peak_markers_round_trips() {
        let mut app = Spectrix::default();
        let histogrammer = &mut app.sessions[0].histogrammer;
        histogrammer.add_hist1d("energy", 32, (0.0, 32.0));
        let pane_id = histogrammer
            .find_existing_histogram("energy")
            .expect("created histogram");
        let Some(egui_tiles::Tile::Pane(Pane::Histogram(histogram))) =
            histogrammer.tree.tiles.get(pane_id)
        else {
            panic!("expected 1D histogram pane");
        };
        let mut histogram = histogram.lock().expect("histogram lock");
        histogram.bins[12] = 41;
        histogram.plot_settings.markers.add_peak_marker(12.5);
        drop(histogram);

        let serialized = ron::ser::to_string(&app).expect("workspace serializes");
        let restored: Spectrix = ron::from_str(&serialized).expect("workspace deserializes");
        let histogrammer = &restored.sessions[0].histogrammer;
        let pane_id = histogrammer
            .find_existing_histogram("energy")
            .expect("restored histogram");
        let Some(egui_tiles::Tile::Pane(Pane::Histogram(histogram))) =
            histogrammer.tree.tiles.get(pane_id)
        else {
            panic!("expected restored 1D histogram pane");
        };
        let histogram = histogram.lock().expect("restored histogram lock");
        assert_eq!(histogram.bins[12], 41);
        assert_eq!(histogram.plot_settings.markers.peak_markers.len(), 1);
        assert_eq!(
            histogram.plot_settings.markers.peak_markers[0]
                .center
                .x_value,
            12.5
        );
    }
}
