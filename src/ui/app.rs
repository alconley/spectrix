use crate::util::processer::Processor;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Spectrix {
    sessions: Vec<Processor>,
    current_session: usize,
}

impl Default for Spectrix {
    fn default() -> Self {
        Self {
            sessions: vec![Processor::new()],
            current_session: 0,
        }
    }
}

impl Spectrix {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
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
            egui::menu::bar(ui, |ui| {
                egui::global_theme_preference_switch(ui);
                ui.heading("Spectrix");
                ui.separator();

                // Session tab switcher
                for (i, _) in self.sessions.iter().enumerate() {
                    if ui
                        .selectable_label(self.current_session == i, format!("Session {}", i + 1))
                        .clicked()
                    {
                        self.current_session = i;
                    }
                }

                // Add new session button
                if ui.button("➕ New Session").clicked() {
                    self.sessions.push(Processor::new());
                    self.current_session = self.sessions.len() - 1;
                }

                ui.add_space(ui.available_width() - 100.0);

                // if there are more than 1 sessions say "Remove Current Session" else say "Reset to Default"
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

        // Draw the UI for the current session
        if let Some(current) = self.sessions.get_mut(self.current_session) {
            egui::TopBottomPanel::top("spectrix_top_menu_panel").show(ctx, |ui| {
                current.histogrammer.menu_ui(ui);
            });

            current.ui(ctx);
        }
    }
}
