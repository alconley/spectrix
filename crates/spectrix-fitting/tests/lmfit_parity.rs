use serde_json::Value;
use spectrix_fitting::{BackgroundCoupling, BackgroundKind, FitOptions, PeakFitRequest, fit_peaks};

const VALUE_RTOL: f64 = 1.0e-8;
const VALUE_ATOL: f64 = 1.0e-10;
const UNCERTAINTY_RTOL: f64 = 1.0e-6;
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
    let result = fit_peaks(
        &PeakFitRequest {
            x,
            y,
            bin_width: 0.1,
            region: [2.0, 8.0],
            peak_markers: vec![5.0],
            background_markers: vec![(2.0, 3.0), (7.0, 8.0)],
            background: BackgroundKind::Linear,
            background_seed: None,
            background_coupling: BackgroundCoupling::PrefitFrozen,
            equal_sigma: true,
            free_centers: true,
            sigma_bounds: None,
        },
        &FitOptions {
            evaluation_x: Some(evaluation_x),
            ..FitOptions::default()
        },
    )
    .expect("native fit succeeds");

    assert!(result.fit.termination.success);
    let values = oracle["parameter_values"].as_object().expect("value map");
    for (name, expected) in values {
        let actual = result
            .fit
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

    let statistics = &result.fit.statistics;
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
            result.fit.best_fit[index],
            number(&oracle["best_fit_samples"][sample]),
            VALUE_RTOL,
            VALUE_ATOL,
            "best-fit curve",
        );
        close(
            result.fit.residuals[index],
            number(&oracle["residual_samples"][sample]),
            VALUE_RTOL,
            VALUE_ATOL,
            "residual",
        );
        close(
            result
                .fit
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

    let covariance = result.fit.covariance.as_ref().expect("covariance");
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
                UNCERTAINTY_ATOL,
                "correlation",
            );
        }
    }
}
