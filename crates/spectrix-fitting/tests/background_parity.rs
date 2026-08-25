use serde_json::Value;
use spectrix_fitting::{
    ConstantModel, ExponentialModel, FitOptions, FitProblem, LinearModel, Model, PowerLawModel,
    QuadraticModel, fit,
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

fn model(case: &Value) -> Box<dyn Model> {
    let initial = &case["initial"];
    match case["name"].as_str().expect("case name") {
        "constant" => Box::new(ConstantModel::new("", [number(&initial["c"])])),
        "linear" => Box::new(LinearModel::new(
            "",
            [number(&initial["slope"]), number(&initial["intercept"])],
        )),
        "quadratic" => Box::new(QuadraticModel::new(
            "",
            [
                number(&initial["a"]),
                number(&initial["b"]),
                number(&initial["c"]),
            ],
        )),
        "exponential" => Box::new(ExponentialModel::new(
            "",
            [number(&initial["amplitude"]), number(&initial["decay"])],
        )),
        "power_law" => Box::new(PowerLawModel::new(
            "",
            [number(&initial["amplitude"]), number(&initial["exponent"])],
        )),
        name => panic!("unknown fixture model {name}"),
    }
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "each background parity case checks all public numerical result categories"
)]
fn all_v1_backgrounds_match_lmfit_134() {
    let oracle: Value = serde_json::from_str(include_str!("parity/lmfit134_backgrounds.json"))
        .expect("valid committed background oracle");
    for case in oracle["cases"].as_array().expect("background cases") {
        let name = case["name"].as_str().expect("case name");
        let x = numbers(&case["x"]);
        let y = numbers(&case["y"]);
        let result = fit(
            &FitProblem::new(model(case), x.clone(), y),
            &FitOptions {
                evaluation_x: Some(x),
                ..FitOptions::default()
            },
        )
        .unwrap_or_else(|error| panic!("native {name} fit failed: {error}"));
        assert!(result.termination.success, "{name} termination");

        for (parameter, expected) in case["values"].as_object().expect("values") {
            let actual = result
                .parameters
                .iter()
                .find(|estimate| &estimate.name == parameter)
                .unwrap_or_else(|| panic!("missing {name} parameter {parameter}"));
            close(
                actual.value,
                number(expected),
                VALUE_RTOL,
                VALUE_ATOL,
                &format!("{name} {parameter}"),
            );
            close(
                actual.standard_error.expect("standard error"),
                number(&case["errors"][parameter]),
                UNCERTAINTY_RTOL,
                UNCERTAINTY_ATOL,
                &format!("{name} {parameter} error"),
            );
        }

        for (statistic, actual) in [
            ("chi_square", result.statistics.chi_square),
            ("reduced_chi_square", result.statistics.reduced_chi_square),
            ("aic", result.statistics.aic.expect("AIC")),
            ("bic", result.statistics.bic.expect("BIC")),
            ("r_squared", result.statistics.r_squared.expect("R-squared")),
        ] {
            close(
                actual,
                number(&case["statistics"][statistic]),
                VALUE_RTOL,
                VALUE_ATOL,
                &format!("{name} {statistic}"),
            );
        }

        for (sample, index) in case["sample_indices"]
            .as_array()
            .expect("sample indices")
            .iter()
            .enumerate()
        {
            let index = index.as_u64().expect("sample index") as usize;
            close(
                result.best_fit[index],
                number(&case["best_fit_samples"][sample]),
                VALUE_RTOL,
                VALUE_ATOL,
                &format!("{name} curve"),
            );
            close(
                result.residuals[index],
                number(&case["residual_samples"][sample]),
                VALUE_RTOL,
                VALUE_ATOL,
                &format!("{name} residual"),
            );
            close(
                result
                    .confidence_band
                    .as_ref()
                    .expect("confidence band")
                    .uncertainty[index],
                number(&case["uncertainty_samples"][sample]),
                UNCERTAINTY_RTOL,
                UNCERTAINTY_ATOL,
                &format!("{name} band"),
            );
        }

        let covariance = result.covariance.as_ref().expect("covariance");
        let expected_names = case["variable_names"]
            .as_array()
            .expect("variable names")
            .iter()
            .map(|value| value.as_str().expect("variable name").to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            covariance.parameter_names, expected_names,
            "{name} variables"
        );
        for row in 0..covariance.matrix.len() {
            for column in 0..covariance.matrix.len() {
                close(
                    covariance.matrix[row][column],
                    number(&case["covariance"][row][column]),
                    UNCERTAINTY_RTOL,
                    UNCERTAINTY_ATOL,
                    &format!("{name} covariance"),
                );
                close(
                    covariance.correlations[row][column],
                    number(&case["correlations"][row][column]),
                    UNCERTAINTY_RTOL,
                    UNCERTAINTY_ATOL,
                    &format!("{name} correlation"),
                );
            }
        }
    }
}
