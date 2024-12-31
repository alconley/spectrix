use crate::util::processer::Processor;
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Spectrix {
    processor: Processor,
}

impl Default for Spectrix {
    fn default() -> Self {
        Self {
            processor: Processor::new(),
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
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("spectrix_top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::global_theme_preference_switch(ui);

                ui.heading("Spectrix");

                ui.separator();

                self.processor.histogrammer.menu_ui(ui);

                ui.add_space(ui.available_width() - 50.0);

                if ui.button("Reset").clicked() {
                    self.reset_to_default();
                }
            });
        });

        self.processor.ui(ctx);
    }
}
