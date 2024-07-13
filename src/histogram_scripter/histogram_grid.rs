#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct GridConfig {
    pub name: String,
    pub histograms: Vec<String>,
    pub selected_histogram: String,
}

impl GridConfig {
    pub fn ui(&mut self, ui: &mut egui::Ui, histo_keys: &[String]) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.name);
            });

            ui.separator();

            ui.label("Histograms:");
            for name in &self.histograms {
                ui.label(name);
            }

            ui.separator();

            egui::ComboBox::from_label("Histogram")
                .selected_text(&self.selected_histogram)
                .show_ui(ui, |ui| {
                    for key in histo_keys {
                        ui.selectable_value(&mut self.selected_histogram, key.clone(), key);
                    }
                });
        });
    }
}
