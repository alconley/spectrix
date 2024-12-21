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
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for Spectrix {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keybinds
        // let input = ctx.input(|state| state.clone()); // Get the input state
        // if input.key_pressed(egui::Key::Tab) {
        //     self.processor.settings.dialog_open = !self.processor.settings.dialog_open;
        // }

        egui::TopBottomPanel::top("spectrix_top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                egui::global_theme_preference_switch(ui);

                ui.heading("Spectrix");

                ui.separator();

                if ui.button("Reset").clicked() {
                    self.processor.reset();
                }

                ui.separator();

                self.processor.histogrammer.menu_ui(ui);
            });
        });

        self.processor.ui(ctx);
    }
}
