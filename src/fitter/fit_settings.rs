use super::main_fitter::Model;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FitSettings {
    pub show_decomposition: bool,
    pub show_composition: bool,
    pub show_background: bool,
    pub show_fit_stats: bool,
    pub fit_stats_height: f32,
    pub equal_stddev: bool,
    pub free_position: bool,
    pub background_model: Model,
}

impl Default for FitSettings {
    fn default() -> Self {
        FitSettings {
            show_decomposition: true,
            show_composition: true,
            show_background: true,
            show_fit_stats: false,
            fit_stats_height: 0.0,
            equal_stddev: true,
            free_position: true,
            background_model: Model::Linear,
        }
    }
}

impl FitSettings {
    pub fn menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Fit Stats: ");
            ui.checkbox(&mut self.show_fit_stats, "Show")
                .on_hover_text("Show the fit statistics above the histogram");

            ui.add(
                egui::DragValue::new(&mut self.fit_stats_height)
                    .speed(1.0)
                    .range(0.0..=f32::INFINITY)
                    .prefix("Height: ")
                    .suffix(" px"),
            )
            .on_hover_text("Set the height of the fit statistics grid to see more fits at once");
        });

        ui.separator();

        ui.horizontal(|ui| {
            ui.label("Show Fit Lines: ");
            ui.checkbox(&mut self.show_decomposition, "Decomposition")
                .on_hover_text("Show the decomposition peaks");
            ui.checkbox(&mut self.show_composition, "Composition")
                .on_hover_text("Show the composition line");
            ui.checkbox(&mut self.show_background, "Background")
                .on_hover_text("Show the background line");
        });

        ui.separator();

        ui.heading("Gaussian Fit Settings");
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.equal_stddev, "Free Standard Deviation")
                .on_hover_text("Allow the standard deviation of the Gaussian to be free");
            ui.checkbox(&mut self.free_position, "Free Position")
                .on_hover_text("Allow the position of the Gaussian to be free");
        });

        ui.separator();

        ui.heading("Background Fit Models");
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.background_model, Model::Linear, "Linear");
            ui.radio_value(&mut self.background_model, Model::None, "None");
        });
        
    }
}
