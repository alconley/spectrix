use super::main_fitter::FitModel;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FitSettings {
    pub show_decomposition: bool,
    pub show_composition: bool,
    pub show_background: bool,
    pub show_fit_stats: bool,
    pub fit_stats_height: f32,
    pub background_model: FitModel,
    pub background_poly_degree: usize,
    pub background_single_exp_initial_guess: f64,
    pub background_double_exp_initial_guess: (f64, f64),
}

impl Default for FitSettings {
    fn default() -> Self {
        FitSettings {
            show_decomposition: true,
            show_composition: true,
            show_background: true,
            show_fit_stats: false,
            fit_stats_height: 0.0,
            background_model: FitModel::Polynomial(1),
            background_poly_degree: 1,
            background_single_exp_initial_guess: 200.0,
            background_double_exp_initial_guess: (200.0, 800.0),
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

        ui.label("Background Fit Models");
        ui.label("Polynomial");
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.background_model,
                FitModel::Polynomial(1),
                "Linear",
            );
            ui.radio_value(
                &mut self.background_model,
                FitModel::Polynomial(2),
                "Quadratic",
            );
            ui.radio_value(&mut self.background_model, FitModel::Polynomial(3), "Cubic");
            ui.radio_value(
                &mut self.background_model,
                FitModel::Polynomial(self.background_poly_degree),
                "n",
            );
            ui.add(
                egui::DragValue::new(&mut self.background_poly_degree)
                    .speed(1)
                    .prefix("Degree: ")
                    .range(1.0..=f32::INFINITY),
            );
        });

        ui.label("Exponential");
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.background_model,
                FitModel::Exponential(self.background_single_exp_initial_guess),
                "Single",
            );

            ui.add(
                egui::DragValue::new(&mut self.background_single_exp_initial_guess)
                    .speed(10)
                    .prefix("b: "),
            );

            ui.radio_value(
                &mut self.background_model,
                FitModel::DoubleExponential(
                    self.background_double_exp_initial_guess.0,
                    self.background_double_exp_initial_guess.1,
                ),
                "Double",
            );

            ui.add(
                egui::DragValue::new(&mut self.background_double_exp_initial_guess.0)
                    .speed(10)
                    .prefix("b: ")
                    .range(0.0..=f64::INFINITY),
            );

            ui.add(
                egui::DragValue::new(&mut self.background_double_exp_initial_guess.1)
                    .speed(10)
                    .prefix("d: ")
                    .range(0.0..=f64::INFINITY),
            );
        });

        ui.separator();
    }
}
