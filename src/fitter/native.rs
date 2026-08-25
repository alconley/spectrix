//! Adapters between the UI's legacy storage structs and `spectrix-fitting`.

use spectrix_fitting::{
    BackgroundKind, BackgroundSeed, Bound, Bounds, FitResult as NativeFitResult,
    ParameterDefinition,
};

use super::{
    common::{Data, Parameter},
    main_fitter::{BackgroundModel, BackgroundResult},
    models::{
        exponential::ExponentialFitter, linear::LinearFitter, powerlaw::PowerLawFitter,
        quadratic::QuadraticFitter,
    },
};

pub(crate) fn background_kind(model: &BackgroundModel) -> BackgroundKind {
    match model {
        BackgroundModel::Linear(_) => BackgroundKind::Linear,
        BackgroundModel::Quadratic(_) => BackgroundKind::Quadratic,
        BackgroundModel::PowerLaw(_) => BackgroundKind::PowerLaw,
        BackgroundModel::Exponential(_) => BackgroundKind::Exponential,
        BackgroundModel::None => BackgroundKind::None,
    }
}

pub(crate) fn background_seed(
    model: &BackgroundModel,
    previous: Option<&BackgroundResult>,
) -> BackgroundSeed {
    let parameters = match model {
        BackgroundModel::Linear(parameters) => vec![
            parameter_definition(
                "bg_slope",
                &parameters.slope,
                previous.and_then(|result| match result {
                    BackgroundResult::Linear(fit) => fit.paramaters.slope.value,
                    _ => None,
                }),
            ),
            parameter_definition(
                "bg_intercept",
                &parameters.intercept,
                previous.and_then(|result| match result {
                    BackgroundResult::Linear(fit) => fit.paramaters.intercept.value,
                    _ => None,
                }),
            ),
        ],
        BackgroundModel::Quadratic(parameters) => vec![
            parameter_definition(
                "bg_a",
                &parameters.a,
                previous.and_then(|result| match result {
                    BackgroundResult::Quadratic(fit) => fit.paramaters.a.value,
                    _ => None,
                }),
            ),
            parameter_definition(
                "bg_b",
                &parameters.b,
                previous.and_then(|result| match result {
                    BackgroundResult::Quadratic(fit) => fit.paramaters.b.value,
                    _ => None,
                }),
            ),
            parameter_definition(
                "bg_c",
                &parameters.c,
                previous.and_then(|result| match result {
                    BackgroundResult::Quadratic(fit) => fit.paramaters.c.value,
                    _ => None,
                }),
            ),
        ],
        BackgroundModel::Exponential(parameters) => vec![
            parameter_definition(
                "bg_amplitude",
                &parameters.amplitude,
                previous.and_then(|result| match result {
                    BackgroundResult::Exponential(fit) => fit.paramaters.amplitude.value,
                    _ => None,
                }),
            ),
            parameter_definition(
                "bg_decay",
                &parameters.decay,
                previous.and_then(|result| match result {
                    BackgroundResult::Exponential(fit) => fit.paramaters.decay.value,
                    _ => None,
                }),
            ),
        ],
        BackgroundModel::PowerLaw(parameters) => vec![
            parameter_definition(
                "bg_amplitude",
                &parameters.amplitude,
                previous.and_then(|result| match result {
                    BackgroundResult::PowerLaw(fit) => fit.paramaters.amplitude.value,
                    _ => None,
                }),
            ),
            parameter_definition(
                "bg_exponent",
                &parameters.exponent,
                previous.and_then(|result| match result {
                    BackgroundResult::PowerLaw(fit) => fit.paramaters.exponent.value,
                    _ => None,
                }),
            ),
        ],
        BackgroundModel::None => vec![ParameterDefinition::fixed("bg_c", 0.0)],
    };
    BackgroundSeed { parameters }
}

pub(crate) fn background_result_from_native(
    model: &BackgroundModel,
    result: &NativeFitResult,
    data: Data,
) -> Option<BackgroundResult> {
    let fit_points = result
        .evaluation_x
        .iter()
        .copied()
        .zip(result.best_fit.iter().copied())
        .map(Into::into)
        .collect::<Vec<_>>();
    let report = fit_report(result);
    match model {
        BackgroundModel::Linear(parameters) => {
            let mut parameters = parameters.clone();
            apply_estimate(&mut parameters.slope, result, "bg_slope");
            apply_estimate(&mut parameters.intercept, result, "bg_intercept");
            Some(BackgroundResult::Linear(LinearFitter {
                data,
                paramaters: parameters,
                fit_points,
                fit_report: report,
            }))
        }
        BackgroundModel::Quadratic(parameters) => {
            let mut parameters = parameters.clone();
            apply_estimate(&mut parameters.a, result, "bg_a");
            apply_estimate(&mut parameters.b, result, "bg_b");
            apply_estimate(&mut parameters.c, result, "bg_c");
            Some(BackgroundResult::Quadratic(QuadraticFitter {
                data,
                paramaters: parameters,
                fit_points,
                fit_report: report,
                covar: covariance_3(result, ["bg_a", "bg_b", "bg_c"]),
            }))
        }
        BackgroundModel::Exponential(parameters) => {
            let mut parameters = parameters.clone();
            apply_estimate(&mut parameters.amplitude, result, "bg_amplitude");
            apply_estimate(&mut parameters.decay, result, "bg_decay");
            Some(BackgroundResult::Exponential(ExponentialFitter {
                data,
                paramaters: parameters,
                fit_points,
                fit_report: report,
            }))
        }
        BackgroundModel::PowerLaw(parameters) => {
            let mut parameters = parameters.clone();
            apply_estimate(&mut parameters.amplitude, result, "bg_amplitude");
            apply_estimate(&mut parameters.exponent, result, "bg_exponent");
            Some(BackgroundResult::PowerLaw(PowerLawFitter {
                data,
                paramaters: parameters,
                fit_points,
                fit_report: report,
            }))
        }
        BackgroundModel::None => None,
    }
}

pub(crate) fn fit_report(result: &NativeFitResult) -> String {
    let statistics = &result.statistics;
    let mut report = format!(
        "[[Native least-squares fit]]\n\
         success = {}\n\
         termination = {} ({})\n\
         function evaluations = {}\n\
         data points = {}\n\
         variables = {}\n\
         chi-square = {:.15e}\n\
         reduced chi-square = {:.15e}\n\
         Akaike information criterion = {}\n\
         Bayesian information criterion = {}\n\
         R-squared = {}\n\
         [[Variables]]\n",
        result.termination.success,
        result.termination.reason,
        result.termination.message,
        statistics.evaluations,
        statistics.observations,
        statistics.variables,
        statistics.chi_square,
        statistics.reduced_chi_square,
        optional_number(statistics.aic),
        optional_number(statistics.bic),
        optional_number(statistics.r_squared),
    );
    for parameter in &result.parameters {
        let error = parameter
            .standard_error
            .map_or_else(|| "unavailable".to_owned(), |value| format!("{value:.15e}"));
        report.push_str(&format!(
            "    {}: {:.15e} +/- {} ({:?})\n",
            parameter.name, parameter.value, error, parameter.kind
        ));
    }
    report
}

fn optional_number(value: Option<f64>) -> String {
    value.map_or_else(|| "unavailable".to_owned(), |value| format!("{value:.15e}"))
}

pub(crate) fn parameter_definition(
    name: &str,
    parameter: &Parameter,
    value: Option<f64>,
) -> ParameterDefinition {
    ParameterDefinition {
        name: name.to_owned(),
        initial: value.unwrap_or(parameter.initial_guess),
        bounds: bounds(parameter.min, parameter.max),
        vary: parameter.vary,
        binding: None,
    }
}

fn bounds(minimum: f64, maximum: f64) -> Bounds {
    Bounds {
        lower: if minimum.is_finite() {
            Bound::Inclusive(minimum)
        } else {
            Bound::Unbounded
        },
        upper: if maximum.is_finite() {
            Bound::Inclusive(maximum)
        } else {
            Bound::Unbounded
        },
    }
}

pub(crate) fn apply_estimate(parameter: &mut Parameter, result: &NativeFitResult, name: &str) {
    if let Some(estimate) = result
        .parameters
        .iter()
        .find(|parameter| parameter.name == name)
    {
        parameter.value = Some(estimate.value);
        parameter.uncertainty = estimate.standard_error;
    }
}

pub(crate) fn covariance_3(result: &NativeFitResult, names: [&str; 3]) -> Option<[[f64; 3]; 3]> {
    let covariance = result.covariance.as_ref()?;
    let indices = names.map(|name| {
        covariance
            .parameter_names
            .iter()
            .position(|candidate| candidate == name)
    });
    let [Some(first), Some(second), Some(third)] = indices else {
        return None;
    };
    let indices = [first, second, third];
    Some(std::array::from_fn(|row| {
        std::array::from_fn(|column| covariance.matrix[indices[row]][indices[column]])
    }))
}
