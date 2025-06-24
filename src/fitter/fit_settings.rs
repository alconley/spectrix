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
            background_model: BackgroundModel::Linear(LinearParameters::default()),
            linear_params: LinearParameters::default(),
            quadratic_params: QuadraticParameters::default(),
            power_law_params: PowerLawParameters::default(),
            exponential_params: ExponentialParameters::default(),
        }
    }
}

impl FitSettings {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // ui.horizontal(|ui| {
        //     ui.label("Fit Panel: ");
        //     ui.checkbox(&mut self.show_fit_stats, "Show")
        //         .on_hover_text("Show the fit statistics above the histogram");
        // });

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
            ui.checkbox(&mut self.equal_stddev, "Equal Standard Deviation")
                .on_hover_text("Allow the standard deviation of the Gaussian to be free");
            ui.checkbox(&mut self.free_position, "Free Position")
                .on_hover_text("Allow the position of the Gaussian to be free");
        });

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
