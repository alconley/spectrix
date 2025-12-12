use crate::util::processer::Processor;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Spectrix {
    sessions: Vec<Processor>,
    session_names: Vec<String>,
    current_session: usize,
}

impl Default for Spectrix {
    fn default() -> Self {
        Self {
            sessions: vec![Processor::new()],
            session_names: vec!["Session 1".to_owned()], // 👈 NEW
            current_session: 0,
        }
    }
}

impl Spectrix {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            let mut app: Self = eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();

            // Make sure session_names is in sync with sessions
            if app.session_names.len() != app.sessions.len() {
                app.session_names = (0..app.sessions.len())
                    .map(|i| format!("Session {}", i + 1))
                    .collect();
            }

            return app;
        }
        Default::default()
    }

    pub fn reset_to_default(&mut self) {
        *self = Default::default();
    }
}

impl eframe::App for Spectrix {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("spectrix_top_panel").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("spectrix_top_scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // --- Left side: logo / tabs / new session ---
                            egui::global_theme_preference_switch(ui);
                            ui.heading("Spectrix");
                            ui.separator();

                            ui.label("Sessions:");
                            for (i, _) in self.sessions.iter().enumerate() {
                                let label = self
                                    .session_names
                                    .get(i)
                                    .map(String::as_str)
                                    .unwrap_or_else(|| "Session");

                                if ui
                                    .selectable_label(self.current_session == i, label.to_owned())
                                    .clicked()
                                {
                                    self.current_session = i;
                                }

                                ui.separator();
                            }

                            if ui.button("➕ New Session").clicked() {
                                self.sessions.push(Processor::new());
                                self.session_names
                                    .push(format!("Session {}", self.sessions.len()));
                                self.current_session = self.sessions.len() - 1;
                            }

                            ui.add_space(50.0);

                            ui.separator();

                            ui.add_space(50.0);

                            // --- Right side: name + remove/reset ---
                            if let Some(name) = self.session_names.get_mut(self.current_session) {
                                ui.label("Session name:");
                                ui.text_edit_singleline(name);
                            }

                            if self.sessions.len() > 1 {
                                if ui.button("Remove Session").clicked() {
                                    self.sessions.remove(self.current_session);
                                    if self.current_session >= self.sessions.len() {
                                        self.current_session = self.sessions.len() - 1;
                                    }
                                }
                            } else if ui.button("Reset Session").clicked() {
                                self.reset_to_default();
                            }
                        });
                    });
            });
        });

        // Draw the UI for the current session
        if let Some(current) = self.sessions.get_mut(self.current_session) {
            egui::TopBottomPanel::top("spectrix_top_menu_panel").show(ctx, |ui| {
                current.histogrammer.menu_ui(ui);
            });

            current.ui(ctx);
        }
    }
}
