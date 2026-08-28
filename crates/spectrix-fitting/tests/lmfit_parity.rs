use serde_json::Value;
use spectrix_fitting::{
    CompositeModel, FitOptions, FitProblem, GaussianModel, LinearModel, ModelComponent,
    ParameterDefinition, fit,
};

// The production initializer now uses finite, data-informed peak bounds instead of lmfit's
// unbounded raw-count guess, so the same minimum can differ by a few final solver digits.
const VALUE_RTOL: f64 = 1.0e-5;
const VALUE_ATOL: f64 = 1.0e-10;
const UNCERTAINTY_RTOL: f64 = 1.0e-4;
const UNCERTAINTY_ATOL: f64 = 1.0e-9;

fn synthetic_data() -> (Vec<f64>, Vec<f64>) {
    let x = (0..=100)
        .map(|index| index as f64 * 0.1)
        .collect::<Vec<_>>();
    let y = x
        .iter()
        .map(|independent| {
            let gaussian = 120.0 / ((2.0 * std::f64::consts::PI).sqrt() * 0.37)
                * (-0.5 * ((*independent - 5.1) / 0.37).powi(2)).exp();
            0.7 * independent
                + 2.0
                + gaussian
                + 0.15 * (1.7 * independent).sin()
                + 0.03 * (4.2 * independent).cos()
        })
        .collect();
    (x, y)
}

fn close(actual: f64, expected: f64, rtol: f64, atol: f64, context: &str) {
    let tolerance = atol + rtol * expected.abs();
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
    );
}

fn number(value: &Value) -> f64 {
    value.as_f64().expect("fixture number")
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "the parity assertion intentionally checks every published result category"
)]
fn gaussian_with_frozen_linear_background_matches_lmfit_134() {
    let oracle: Value = serde_json::from_str(include_str!("parity/lmfit134_gaussian_linear.json"))
        .expect("valid committed oracle");
    let (x, y) = synthetic_data();
    let evaluation_x = x
        .iter()
        .copied()
        .filter(|value| *value >= 2.0 && *value <= 8.0)
        .collect::<Vec<_>>();
    let (fit_x, fit_y): (Vec<_>, Vec<_>) = x
        .into_iter()
        .zip(y)
        .filter(|(value, _)| *value >= 2.0 && *value <= 8.0)
        .unzip();
    let background = LinearModel::new("bg_", [0.7, 2.0]).with_parameters([
        ParameterDefinition::fixed("bg_slope", number(&oracle["parameter_values"]["bg_slope"])),
        ParameterDefinition::fixed(
            "bg_intercept",
            number(&oracle["parameter_values"]["bg_intercept"]),
        ),
    ]);
    let gaussian = GaussianModel::new("g0_", 120.0, 5.0, 0.4).with_bin_width(0.1);
    let mut model = CompositeModel::default();
    model
        .push(ModelComponent::new("background", Box::new(background)))
        .expect("background component");
    model
        .push(ModelComponent::new("g0_", Box::new(gaussian)))
        .expect("Gaussian component");
    let result = fit(
        &FitProblem::new(Box::new(model), fit_x, fit_y),
        &FitOptions {
            evaluation_x: Some(evaluation_x),
            ..FitOptions::default()
        },
    )
    .expect("native fit succeeds");

    assert!(result.termination.success);
    let values = oracle["parameter_values"].as_object().expect("value map");
    for (name, expected) in values {
        let actual = result
            .parameters
            .iter()
            .find(|parameter| &parameter.name == name)
            .unwrap_or_else(|| panic!("missing native parameter {name}"));
        close(
            actual.value,
            number(expected),
            VALUE_RTOL,
            VALUE_ATOL,
            &format!("parameter {name}"),
        );
    }

    let statistics = &result.statistics;
    for (name, actual) in [
        ("chi_square", statistics.chi_square),
        ("reduced_chi_square", statistics.reduced_chi_square),
        ("aic", statistics.aic.expect("AIC")),
        ("bic", statistics.bic.expect("BIC")),
        ("r_squared", statistics.r_squared.expect("R-squared")),
    ] {
        close(
            actual,
            number(&oracle["statistics"][name]),
            VALUE_RTOL,
            VALUE_ATOL,
            name,
        );
    }

    let sample_indices = oracle["sample_indices"].as_array().expect("sample indices");
    for (sample, index) in sample_indices.iter().enumerate() {
        let index = index.as_u64().expect("index") as usize;
        close(
            result.best_fit[index],
            number(&oracle["best_fit_samples"][sample]),
            VALUE_RTOL,
            VALUE_ATOL,
            "best-fit curve",
        );
        close(
            result.residuals[index],
            number(&oracle["residual_samples"][sample]),
            VALUE_RTOL,
            VALUE_ATOL,
            "residual",
        );
        close(
            result
                .confidence_band
                .as_ref()
                .expect("confidence band")
                .uncertainty[index],
            number(&oracle["uncertainty_samples"][sample]),
            UNCERTAINTY_RTOL,
            UNCERTAINTY_ATOL,
            "uncertainty band",
        );
    }

    let covariance = result.covariance.as_ref().expect("covariance");
    let expected_names = oracle["variable_names"].as_array().expect("variable names");
    assert_eq!(
        covariance.parameter_names,
        expected_names
            .iter()
            .map(|name| name.as_str().expect("name").to_owned())
            .collect::<Vec<_>>()
    );
    for row in 0..covariance.matrix.len() {
        for column in 0..covariance.matrix.len() {
            close(
                covariance.matrix[row][column],
                number(&oracle["covariance"][row][column]),
                UNCERTAINTY_RTOL,
                UNCERTAINTY_ATOL,
                "covariance",
            );
            close(
                covariance.correlations[row][column],
                number(&oracle["correlations"][row][column]),
                UNCERTAINTY_RTOL,
                1.0e-4,
                "correlation",
            );
        }
    }
}
