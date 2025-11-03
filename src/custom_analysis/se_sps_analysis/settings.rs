use super::integration_methods::IntegrationRule;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SPSAnalysisSettings {
    pub panel_open: bool,
    pub n_columns: usize,
    pub log_scale: bool,
    pub view_aspect: f32,
    pub markersize: f32,
    pub fit_mean: bool,
    pub integration_rule: IntegrationRule,
    pub show_integration: bool,
}

impl Default for SPSAnalysisSettings {
    fn default() -> Self {
        Self {
            panel_open: true,
            n_columns: 3,
            log_scale: true,
            view_aspect: 2.0,
            markersize: 3.0,
            fit_mean: true,
            integration_rule: IntegrationRule::Trapezoidal,
            show_integration: false,
        }
    }
}

impl SPSAnalysisSettings {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("sps_settings_grid")
            .striped(true)
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Number of Columns:");
                ui.add(
                    egui::DragValue::new(&mut self.n_columns)
                        .speed(1)
                        .range(1..=10),
                );
                ui.end_row();
                ui.label("Log Scale:");
                ui.checkbox(&mut self.log_scale, "");
                ui.end_row();
                ui.label("View Aspect Ratio:");
                ui.add(
                    egui::DragValue::new(&mut self.view_aspect)
                        .speed(0.1)
                        .range(0.1..=10.0),
                );
                ui.end_row();
                ui.label("Marker Size:");
                ui.add(
                    egui::DragValue::new(&mut self.markersize)
                        .speed(0.1)
                        .range(0.1..=10.0),
                );
                ui.end_row();
                ui.label("Show Fit Mean:");
                ui.checkbox(&mut self.fit_mean, "");
                ui.end_row();

                ui.label("Integration Rule");
                ui.end_row();

                egui::ComboBox::from_id_salt("Integration Rule")
                    .selected_text(format!("{}", self.integration_rule))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.integration_rule,
                            IntegrationRule::Left,
                            "Left endpoint",
                        );
                        ui.selectable_value(
                            &mut self.integration_rule,
                            IntegrationRule::Right,
                            "Right endpoint",
                        );
                        ui.selectable_value(
                            &mut self.integration_rule,
                            IntegrationRule::Midpoint,
                            "Midpoint",
                        );
                        ui.selectable_value(
                            &mut self.integration_rule,
                            IntegrationRule::Trapezoidal,
                            "Trapezoidal",
                        );
                    });

                ui.checkbox(&mut self.show_integration, "Visualize");

                ui.end_row();
            });
    }
}
