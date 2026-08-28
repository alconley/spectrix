use crate::fitter::common::Parameter;
use crate::fitter::main_fitter::{BackgroundModel, BackgroundResult};
use crate::fitter::models::exponential::ExponentialParameters;
use crate::fitter::models::linear::LinearParameters;
use crate::fitter::models::powerlaw::PowerLawParameters;
use crate::fitter::models::quadratic::QuadraticParameters;
use spectrix_fitting::{BackgroundCoupling, ObjectiveKind};

/// The objective requested by the histogram UI. `Auto` selects least squares
/// for high-count bins, where its Gaussian approximation is more stable, and
/// Poisson deviance otherwise.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum HistogramObjective {
    #[default]
    Auto,
    PoissonDeviance,
    LeastSquares,
}

impl HistogramObjective {
    pub const HIGH_COUNT_THRESHOLD: f64 = 100.0;

    pub fn resolve(self, counts: impl IntoIterator<Item = f64>) -> ObjectiveKind {
        match self {
            Self::PoissonDeviance => ObjectiveKind::PoissonDeviance,
            Self::LeastSquares => ObjectiveKind::LeastSquares,
            Self::Auto => {
                let maximum = counts
                    .into_iter()
                    .filter(|count| count.is_finite())
                    .fold(0.0_f64, f64::max);
                if maximum >= Self::HIGH_COUNT_THRESHOLD {
                    ObjectiveKind::LeastSquares
                } else {
                    ObjectiveKind::PoissonDeviance
                }
            }
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::PoissonDeviance => "Poisson",
            Self::LeastSquares => "Least squares",
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct FitSettings {
    pub show_decomposition: bool,
    pub show_composition: bool,
    pub show_background: bool,
    pub show_fit_lines_area: bool,
    pub uuid_label_size: f32,
    pub uuid_label_lift: f32,
    pub uuid_label_guides: bool,
    pub show_fit_stats: bool,
    pub fit_panel_popout: bool,
    pub equal_stddev: bool,
    pub free_position: bool,
    pub background_model: BackgroundModel,
    pub lock_background: bool,
    /// Legacy persisted coupling mode; active fits derive this from `lock_background`.
    pub background_coupling: BackgroundCoupling,
    pub linear_params: LinearParameters,
    pub quadratic_params: QuadraticParameters,
    pub power_law_params: PowerLawParameters,
    pub exponential_params: ExponentialParameters,
    pub constant_param: Parameter,
    pub objective: HistogramObjective,
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
            show_fit_lines_area: true,
            uuid_label_size: 14.0,
            uuid_label_lift: 1.6,
            uuid_label_guides: true,
            show_fit_stats: false,
            fit_panel_popout: false,
            equal_stddev: true,
            free_position: true,
            background_model: BackgroundModel::None,
            lock_background: false,
            background_coupling: BackgroundCoupling::PrefitFrozen,
            linear_params: LinearParameters::default(),
            quadratic_params: QuadraticParameters::default(),
            power_law_params: PowerLawParameters::default(),
            exponential_params: ExponentialParameters::default(),
            constant_param: Parameter {
                name: "Constant".to_owned(),
                min: 0.0,
                ..Parameter::default()
            },
            objective: HistogramObjective::Auto,
            calibrated: false,
            constrain_sigma: false,
            sigma_min: 0.1,
            sigma_max: 10.0,
        }
    }
}

impl FitSettings {
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        background_parameters_are_current: bool,
        manual_background_available: bool,
    ) {
        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label("Background Models");

            ui.radio_value(
                &mut self.background_model,
                BackgroundModel::Constant(self.constant_param.clone()),
                "Constant",
            );

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

        if let BackgroundModel::Constant(parameter) = &mut self.background_model {
            parameter.ui(ui);
            self.constant_param = parameter.clone();
        }

        if let BackgroundModel::Linear(params) = &mut self.background_model {
            params.ui(ui, background_parameters_are_current);
            self.linear_params = params.clone();
        }

        if let BackgroundModel::Quadratic(params) = &mut self.background_model {
            params.ui(ui, background_parameters_are_current);
            self.quadratic_params = params.clone();
        }

        if let BackgroundModel::PowerLaw(params) = &mut self.background_model {
            params.ui(ui, background_parameters_are_current);
            self.power_law_params = params.clone();
        }

        if let BackgroundModel::Exponential(params) = &mut self.background_model {
            params.ui(ui, background_parameters_are_current);
            self.exponential_params = params.clone();
        }

        if !matches!(
            self.background_model,
            BackgroundModel::None | BackgroundModel::LegacyAuto
        ) {
            ui.add_enabled(
                manual_background_available,
                egui::Checkbox::new(&mut self.lock_background, "Lock manual background"),
            )
            .on_hover_text(if manual_background_available {
                "Keep the manually fitted background fixed in subsequent peak fits. Disable this to use it only as a starting estimate."
            } else {
                "Run an explicit background fit first to make a lockable background available."
            });
            if !manual_background_available {
                self.lock_background = false;
            }
        }

        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label("Gaussian Fit Settings");
            ui.radio_value(&mut self.equal_stddev, true, "Shared FWHM");
            ui.radio_value(&mut self.equal_stddev, false, "Independent FWHM");
            ui.checkbox(&mut self.free_position, "Free Position")
                .on_hover_text("Allow the position of the Gaussian to be free");
        });

        ui.collapsing("Advanced fitting", |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Histogram objective:");
                ui.radio_value(
                    &mut self.objective,
                    HistogramObjective::Auto,
                    "Auto",
                )
                .on_hover_text("Uses least squares when any selected bin has 100 or more counts; otherwise uses Poisson deviance.");
                ui.radio_value(
                    &mut self.objective,
                    HistogramObjective::PoissonDeviance,
                    "Poisson",
                );
                ui.radio_value(
                    &mut self.objective,
                    HistogramObjective::LeastSquares,
                    "Least squares",
                );
            });
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
            ui.checkbox(&mut self.show_fit_lines_area, "1σ Uncertainty")
                .on_hover_text(
                    "Draw the covariance-based, Student-t-scaled total-fit 1σ uncertainty band.",
                );
        });

        ui.horizontal_wrapped(|ui| {
            ui.label("UUID Labels:");
            ui.add(egui::Slider::new(&mut self.uuid_label_size, 8.0..=32.0).text("Size"))
                .on_hover_text(
                    "Adjust the UUID label size drawn above the fitted composition peaks.",
                );
            ui.add(egui::Slider::new(&mut self.uuid_label_lift, 0.0..=3.0).text("Lift"))
                .on_hover_text(
                    "Move UUID labels closer to or farther above their reference height.",
                );
            ui.checkbox(&mut self.uuid_label_guides, "Guide")
                .on_hover_text(
                    "Draw a dashed vertical guide from the bottom of the UUID label to its zero-lift reference height.",
                );
        });

        ui.separator();
    }

    pub fn apply_background_fit(&mut self, result: &BackgroundResult) {
        fn update(target: &mut Parameter, fitted: &Parameter) {
            if let Some(value) = fitted.value.filter(|value| value.is_finite()) {
                target.initial_guess = value;
            }
        }

        match (&mut self.background_model, result) {
            (BackgroundModel::Constant(parameter), BackgroundResult::Constant(fit)) => {
                update(parameter, &fit.paramaters.intercept);
                self.constant_param = parameter.clone();
            }
            (BackgroundModel::Linear(parameters), BackgroundResult::Linear(fit)) => {
                update(&mut parameters.slope, &fit.paramaters.slope);
                update(&mut parameters.intercept, &fit.paramaters.intercept);
                self.linear_params = parameters.clone();
            }
            (BackgroundModel::Quadratic(parameters), BackgroundResult::Quadratic(fit)) => {
                update(&mut parameters.a, &fit.paramaters.a);
                update(&mut parameters.b, &fit.paramaters.b);
                update(&mut parameters.c, &fit.paramaters.c);
                self.quadratic_params = parameters.clone();
            }
            (BackgroundModel::PowerLaw(parameters), BackgroundResult::PowerLaw(fit)) => {
                update(&mut parameters.amplitude, &fit.paramaters.amplitude);
                update(&mut parameters.exponent, &fit.paramaters.exponent);
                self.power_law_params = parameters.clone();
            }
            (BackgroundModel::Exponential(parameters), BackgroundResult::Exponential(fit)) => {
                update(&mut parameters.amplitude, &fit.paramaters.amplitude);
                update(&mut parameters.decay, &fit.paramaters.decay);
                self.exponential_params = parameters.clone();
            }
            _ => {}
        }
    }

    pub fn background_parameters_match(&self, result: &BackgroundResult) -> bool {
        let matches = |initial_guess: f64, fitted: &Parameter| {
            fitted
                .value
                .is_some_and(|value| value.is_finite() && initial_guess == value)
        };
        match (&self.background_model, result) {
            (BackgroundModel::Constant(parameter), BackgroundResult::Constant(fit)) => {
                matches(parameter.initial_guess, &fit.paramaters.intercept)
            }
            (BackgroundModel::Linear(parameters), BackgroundResult::Linear(fit)) => {
                matches(parameters.slope.initial_guess, &fit.paramaters.slope)
                    && matches(
                        parameters.intercept.initial_guess,
                        &fit.paramaters.intercept,
                    )
            }
            (BackgroundModel::Quadratic(parameters), BackgroundResult::Quadratic(fit)) => {
                matches(parameters.a.initial_guess, &fit.paramaters.a)
                    && matches(parameters.b.initial_guess, &fit.paramaters.b)
                    && matches(parameters.c.initial_guess, &fit.paramaters.c)
            }
            (BackgroundModel::PowerLaw(parameters), BackgroundResult::PowerLaw(fit)) => {
                matches(
                    parameters.amplitude.initial_guess,
                    &fit.paramaters.amplitude,
                ) && matches(parameters.exponent.initial_guess, &fit.paramaters.exponent)
            }
            (BackgroundModel::Exponential(parameters), BackgroundResult::Exponential(fit)) => {
                matches(
                    parameters.amplitude.initial_guess,
                    &fit.paramaters.amplitude,
                ) && matches(parameters.decay.initial_guess, &fit.paramaters.decay)
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FitSettings, HistogramObjective};
    use crate::fitter::{
        main_fitter::{BackgroundModel, BackgroundResult},
        models::quadratic::QuadraticFitter,
    };
    #[test]
    fn legacy_equal_stddev_remains_the_width_coupling() {
        let shared: FitSettings =
            serde_json::from_str(r#"{"equal_stddev":true}"#).expect("legacy shared settings");
        let independent: FitSettings =
            serde_json::from_str(r#"{"equal_stddev":false}"#).expect("legacy independent settings");
        assert!(shared.equal_stddev);
        assert!(!independent.equal_stddev);
    }

    #[test]
    fn fitted_background_values_become_visible_next_guesses() {
        let mut settings = FitSettings {
            background_model: BackgroundModel::Quadratic(Default::default()),
            ..FitSettings::default()
        };
        let fitted = BackgroundResult::Quadratic(QuadraticFitter::new_from_parameters(
            (0.125, 0.01),
            (-2.5, 0.02),
            (40.0, 0.5),
            0.0,
            10.0,
        ));

        settings.apply_background_fit(&fitted);

        let BackgroundModel::Quadratic(parameters) = &settings.background_model else {
            panic!("expected quadratic settings");
        };
        assert_eq!(parameters.a.initial_guess, 0.125);
        assert_eq!(parameters.b.initial_guess, -2.5);
        assert_eq!(parameters.c.initial_guess, 40.0);
        assert!(settings.background_parameters_match(&fitted));
        let BackgroundModel::Quadratic(parameters) = &mut settings.background_model else {
            panic!("expected quadratic settings");
        };
        parameters.a.initial_guess = 0.2;
        assert!(!settings.background_parameters_match(&fitted));
    }

    #[test]
    fn auto_objective_uses_least_squares_for_high_count_bins() {
        assert_eq!(
            HistogramObjective::Auto.resolve([0.0, 99.0]),
            spectrix_fitting::ObjectiveKind::PoissonDeviance
        );
        assert_eq!(
            HistogramObjective::Auto.resolve([0.0, 100.0]),
            spectrix_fitting::ObjectiveKind::LeastSquares
        );
    }
}
