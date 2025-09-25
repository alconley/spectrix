use crate::fitter::main_fitter::BackgroundModel;
use crate::fitter::models::exponential::ExponentialParameters;
use crate::fitter::models::linear::LinearParameters;
use crate::fitter::models::powerlaw::PowerLawParameters;
use crate::fitter::models::quadratic::QuadraticParameters;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FitSettings {
    pub show_decomposition: bool,
    pub show_composition: bool,
    pub show_background: bool,
    pub show_fit_stats: bool,
    pub fit_stats_height: f32,
    pub equal_stddev: bool,
    pub free_position: bool,
    pub background_model: BackgroundModel,
    pub linear_params: LinearParameters,
    pub quadratic_params: QuadraticParameters,
    pub power_law_params: PowerLawParameters,
    pub exponential_params: ExponentialParameters,
    pub calibrated: bool,
    pub constrain_sigma: bool,
    pub sigma_min: f64,
    pub sigma_max: f64,
}

impl Default for FitSettings {
    fn default() -> Self {
        Self {
            show_decomposition: true,
            show_composition: true,
            show_background: true,
            show_fit_stats: false,
            fit_stats_height: 0.0,
            equal_stddev: true,
            free_position: true,
            background_model: BackgroundModel::Linear(LinearParameters::default()),
            linear_params: LinearParameters::default(),
            quadratic_params: QuadraticParameters::default(),
            power_law_params: PowerLawParameters::default(),
            exponential_params: ExponentialParameters::default(),
            calibrated: false,
            constrain_sigma: false,
            sigma_min: 0.1,
            sigma_max: 10.0,
        }
    }
}

impl FitSettings {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label("Background Models");

            ui.radio_value(
                &mut self.background_model,
                BackgroundModel::Linear(self.linear_params.clone()),
                "Linear",
            );
            ui.radio_value(
                &mut self.background_model,
                BackgroundModel::Quadratic(self.quadratic_params.clone()),
                "Quadratic",
            );
            ui.radio_value(
                &mut self.background_model,
                BackgroundModel::PowerLaw(self.power_law_params.clone()),
                "Power Law",
            );
            ui.radio_value(
                &mut self.background_model,
                BackgroundModel::Exponential(self.exponential_params.clone()),
                "Exponential",
            );
            ui.radio_value(&mut self.background_model, BackgroundModel::None, "None");
        });

        if let BackgroundModel::Linear(params) = &mut self.background_model {
            params.ui(ui);
            self.linear_params = params.clone();
        }

        if let BackgroundModel::Quadratic(params) = &mut self.background_model {
            params.ui(ui);
            self.quadratic_params = params.clone();
        }

        if let BackgroundModel::PowerLaw(params) = &mut self.background_model {
            params.ui(ui);
            self.power_law_params = params.clone();
        }

        if let BackgroundModel::Exponential(params) = &mut self.background_model {
            params.ui(ui);
            self.exponential_params = params.clone();
        }

        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label("Gaussian Fit Settings");
            ui.checkbox(&mut self.equal_stddev, "Equal σ")
                .on_hover_text("Allow the standard deviation of the Gaussian to be free");
            ui.checkbox(&mut self.free_position, "Free Position")
                .on_hover_text("Allow the position of the Gaussian to be free");
        });

        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.constrain_sigma, "Constrain σ")
                .on_hover_text(
                    "Enable optional lower/upper bounds for σ.\n\
                   If Equal Standard Deviation is ON, a single pair applies to all peaks.\n\
                   If OFF, this pair is broadcast to all peaks.",
                );
            ui.add_enabled_ui(self.constrain_sigma, |ui| {
                ui.label("min:");
                ui.add(egui::DragValue::new(&mut self.sigma_min).speed(0.01));
                ui.label("max:");
                ui.add(egui::DragValue::new(&mut self.sigma_max).speed(0.01));
            });
        });

        // keep min ≤ max (when enabled)
        if self.constrain_sigma && self.sigma_max < self.sigma_min {
            std::mem::swap(&mut self.sigma_min, &mut self.sigma_max);
        }

        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label("Show Fit Lines: ");
            ui.checkbox(&mut self.show_decomposition, "Decomposition")
                .on_hover_text("Show the decomposition peaks");
            ui.checkbox(&mut self.show_composition, "Composition")
                .on_hover_text("Show the composition line");
            ui.checkbox(&mut self.show_background, "Background")
                .on_hover_text("Show the background line");
        });

        ui.separator();
    }
}
