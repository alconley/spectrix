use std::collections::BTreeMap;

use serde_json::Value;
use spectrix_fitting::{
    BackgroundCoupling, BackgroundKind, BackgroundSeed, Bounds, FitOptions, FitResult,
    ParameterDefinition, ParameterKind, PeakFitRequest, SigmaBounds, fit_peaks,
};

const VALUE_RTOL: f64 = 1.0e-8;
const VALUE_ATOL: f64 = 1.0e-10;
const UNCERTAINTY_RTOL: f64 = 1.0e-6;
const UNCERTAINTY_ATOL: f64 = 1.0e-9;

fn number(value: &Value) -> f64 {
    value.as_f64().expect("fixture number")
}

fn numbers(value: &Value) -> Vec<f64> {
    value
        .as_array()
        .expect("fixture array")
        .iter()
        .map(number)
        .collect()
}

fn close(actual: f64, expected: f64, rtol: f64, atol: f64, context: &str) {
    let tolerance = atol + rtol * expected.abs();
    assert!(
        (actual - expected).abs() <= tolerance,
        "{context}: actual={actual:.17e}, expected={expected:.17e}, tolerance={tolerance:.3e}"
    );
}

fn background(value: &Value) -> BackgroundKind {
    match value.as_str().expect("background kind") {
        "none" => BackgroundKind::None,
        "constant" => BackgroundKind::Constant,
        "linear" => BackgroundKind::Linear,
        "quadratic" => BackgroundKind::Quadratic,
        "exponential" => BackgroundKind::Exponential,
        "power_law" => BackgroundKind::PowerLaw,
        name => panic!("unknown fixture background {name}"),
    }
}

fn coupling(value: &Value) -> BackgroundCoupling {
    match value.as_str().expect("background coupling") {
        "frozen" => BackgroundCoupling::PrefitFrozen,
        "joint" => BackgroundCoupling::PrefitJoint,
        name => panic!("unknown fixture coupling {name}"),
    }
}

fn sigma_bounds(value: &Value) -> Option<SigmaBounds> {
    (!value.is_null()).then(|| SigmaBounds {
        minima: numbers(&value["minima"]),
        maxima: numbers(&value["maxima"]),
    })
}

fn background_seed(value: Option<&Value>) -> Option<BackgroundSeed> {
    value.and_then(|value| {
        value.as_array().map(|definitions| BackgroundSeed {
            parameters: definitions
                .iter()
                .map(|definition| {
                    let mut parameter = ParameterDefinition::varying(
                        definition["name"].as_str().expect("seed name"),
                        number(&definition["initial"]),
                    )
                    .with_bounds(Bounds::finite(
                        number(&definition["minimum"]),
                        number(&definition["maximum"]),
                    ));
                    parameter.vary = definition["vary"].as_bool().expect("seed vary");
                    parameter
                })
                .collect(),
        })
    })
}

fn parameter_kind(value: &Value) -> ParameterKind {
    match value.as_str().expect("parameter kind") {
        "free" => ParameterKind::Free,
        "fixed" => ParameterKind::Fixed,
        "shared" => ParameterKind::Shared,
        "derived" => ParameterKind::Derived,
        name => panic!("unknown parameter kind {name}"),
    }
}

fn assert_array(actual: &[f64], expected: &Value, rtol: f64, atol: f64, context: &str) {
    let expected = expected.as_array().expect("expected array");
    assert_eq!(actual.len(), expected.len(), "{context} length");
    for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        close(
            *actual,
            number(expected),
            rtol,
            atol,
            &format!("{context}[{index}]"),
        );
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the parity helper checks every structured result category"
)]
fn assert_result(actual: &FitResult, expected: &Value, case: &str, result_name: &str) {
    let context = |category: &str| format!("{case} {result_name} {category}");
    assert_eq!(
        actual.termination.success,
        expected["success"].as_bool().expect("success"),
        "{}",
        context("success")
    );

    let expected_parameters = expected["parameters"]
        .as_array()
        .expect("expected parameters");
    assert_eq!(
        actual.parameters.len(),
        expected_parameters.len(),
        "{}",
        context("parameter count")
    );
    for expected_parameter in expected_parameters {
        let name = expected_parameter["name"].as_str().expect("parameter name");
        let parameter = actual
            .parameters
            .iter()
            .find(|parameter| parameter.name == name)
            .unwrap_or_else(|| panic!("{} missing parameter {name}", context("parameters")));
        close(
            parameter.value,
            number(&expected_parameter["value"]),
            VALUE_RTOL,
            VALUE_ATOL,
            &context(&format!("parameter {name}")),
        );
        match expected_parameter.get("standard_error") {
            Some(Value::Number(expected_error)) => close(
                parameter
                    .standard_error
                    .unwrap_or_else(|| panic!("{} missing {name} error", context("parameters"))),
                expected_error.as_f64().expect("standard error"),
                UNCERTAINTY_RTOL,
                UNCERTAINTY_ATOL,
                &context(&format!("parameter {name} error")),
            ),
            Some(Value::Null) | None => assert!(
                parameter.standard_error.is_none(),
                "{} expected unavailable {name} error",
                context("parameters")
            ),
            Some(_) => panic!("invalid standard error fixture"),
        }
        assert_eq!(
            parameter.kind,
            parameter_kind(&expected_parameter["kind"]),
            "{}",
            context(&format!("parameter {name} kind"))
        );
        assert_eq!(
            parameter.active_bound,
            expected_parameter["active_bound"]
                .as_bool()
                .expect("active bound"),
            "{}",
            context(&format!("parameter {name} active bound"))
        );
    }

    for (name, value) in [
        ("chi_square", actual.statistics.chi_square),
        ("reduced_chi_square", actual.statistics.reduced_chi_square),
        ("aic", actual.statistics.aic.expect("AIC")),
        ("bic", actual.statistics.bic.expect("BIC")),
        ("r_squared", actual.statistics.r_squared.expect("R-squared")),
    ] {
        close(
            value,
            number(&expected["statistics"][name]),
            VALUE_RTOL,
            VALUE_ATOL,
            &context(name),
        );
    }
    let residual_indices = expected["residual_sample_indices"]
        .as_array()
        .expect("residual sample indices");
    let residual_samples = expected["residual_samples"]
        .as_array()
        .expect("residual samples");
    assert_eq!(
        residual_indices.len(),
        residual_samples.len(),
        "{}",
        context("residual sample count")
    );
    for (index, expected_residual) in residual_indices.iter().zip(residual_samples) {
        let index = index.as_u64().expect("residual index") as usize;
        close(
            actual.residuals[index],
            number(expected_residual),
            VALUE_RTOL,
            VALUE_ATOL,
            &context(&format!("residuals[{index}]")),
        );
    }
    assert_array(
        &actual.best_fit,
        &expected["best_fit"],
        VALUE_RTOL,
        VALUE_ATOL,
        &context("best fit"),
    );

    let components = actual
        .components
        .iter()
        .map(|component| (component.name.as_str(), component.values.as_slice()))
        .collect::<BTreeMap<_, _>>();
    let expected_components = expected["components"].as_object().expect("components");
    assert_eq!(
        components.len(),
        expected_components.len(),
        "{}",
        context("components")
    );
    for (name, expected_values) in expected_components {
        assert_array(
            components
                .get(name.as_str())
                .unwrap_or_else(|| panic!("{} missing {name}", context("components"))),
            expected_values,
            VALUE_RTOL,
            VALUE_ATOL,
            &context(&format!("component {name}")),
        );
    }

    if expected["covariance"].is_null() {
        assert!(
            actual.covariance.is_none(),
            "{}",
            context("covariance availability")
        );
        assert!(
            actual.confidence_band.is_none(),
            "{}",
            context("band availability")
        );
        return;
    }

    let covariance = actual.covariance.as_ref().expect("native covariance");
    let expected_names = expected["variable_names"]
        .as_array()
        .expect("variable names")
        .iter()
        .map(|name| name.as_str().expect("variable name").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        covariance.parameter_names,
        expected_names,
        "{}",
        context("variables")
    );
    for row in 0..covariance.matrix.len() {
        for column in 0..covariance.matrix.len() {
            close(
                covariance.matrix[row][column],
                number(&expected["covariance"][row][column]),
                UNCERTAINTY_RTOL,
                UNCERTAINTY_ATOL,
                &context("covariance"),
            );
            close(
                covariance.correlations[row][column],
                number(&expected["correlations"][row][column]),
                UNCERTAINTY_RTOL,
                UNCERTAINTY_ATOL,
                &context("correlation"),
            );
        }
    }

    let band = actual
        .confidence_band
        .as_ref()
        .expect("native confidence band");
    assert_array(
        &band.uncertainty,
        &expected["uncertainty"],
        UNCERTAINTY_RTOL,
        UNCERTAINTY_ATOL,
        &context("uncertainty band"),
    );
    for (index, ((best, lower), upper)) in band
        .best_fit
        .iter()
        .zip(&band.lower)
        .zip(&band.upper)
        .enumerate()
    {
        close(
            *lower,
            *best - band.uncertainty[index],
            UNCERTAINTY_RTOL,
            UNCERTAINTY_ATOL,
            &context("lower band"),
        );
        close(
            *upper,
            *best + band.uncertainty[index],
            UNCERTAINTY_RTOL,
            UNCERTAINTY_ATOL,
            &context("upper band"),
        );
    }

    let component_bands = actual
        .component_bands
        .iter()
        .map(|(name, band)| (name.as_str(), band))
        .collect::<BTreeMap<_, _>>();
    for (name, expected_values) in expected["component_uncertainties"]
        .as_object()
        .expect("component uncertainties")
    {
        assert_array(
            &component_bands
                .get(name.as_str())
                .unwrap_or_else(|| panic!("{} missing {name}", context("component bands")))
                .uncertainty,
            expected_values,
            UNCERTAINTY_RTOL,
            UNCERTAINTY_ATOL,
            &context(&format!("component {name} band")),
        );
    }
}

#[test]
fn high_level_peak_matrix_matches_lmfit_134() {
    let oracle: Value = serde_json::from_str(include_str!("parity/lmfit134_peak_matrix.json"))
        .expect("valid committed peak matrix oracle");
    for case in oracle["cases"].as_array().expect("peak cases") {
        let name = case["name"].as_str().expect("case name");
        let result = fit_peaks(
            &PeakFitRequest {
                x: numbers(&case["x"]),
                y: numbers(&case["y"]),
                bin_width: 0.1,
                region: [number(&case["region"][0]), number(&case["region"][1])],
                peak_markers: numbers(&case["peak_markers"]),
                background_markers: case["background_markers"]
                    .as_array()
                    .expect("background markers")
                    .iter()
                    .map(|window| (number(&window[0]), number(&window[1])))
                    .collect(),
                background: background(&case["background"]),
                background_seed: background_seed(case.get("background_seed")),
                background_coupling: coupling(&case["coupling"]),
                equal_sigma: case["equal_sigma"].as_bool().expect("equal sigma"),
                free_centers: case["free_centers"].as_bool().expect("free centers"),
                sigma_bounds: sigma_bounds(&case["sigma_bounds"]),
            },
            &FitOptions {
                evaluation_x: Some(numbers(&case["evaluation_x"])),
                ..FitOptions::default()
            },
        )
        .unwrap_or_else(|error| panic!("native {name} fit failed: {error}"));
        assert_eq!(
            result.region,
            [
                number(&case["used_region"][0]),
                number(&case["used_region"][1])
            ]
        );
        assert_eq!(result.peak_markers, numbers(&case["used_peak_markers"]));
        assert_result(&result.fit, &case["fit"], name, "fit");
        assert_result(
            &result.background_prefit,
            &case["background_prefit"],
            name,
            "background prefit",
        );
    }
}
