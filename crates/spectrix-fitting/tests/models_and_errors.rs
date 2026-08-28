use spectrix_fitting::{
    BackgroundCoupling, BackgroundKind, Bounds, CompositeModel, ConstantModel, ExponentialModel,
    FitError, FitOptions, FitProblem, GaussianModel, LinearModel, ManualPeakSeed, Model as _,
    ModelComponent, ObjectiveKind, ParameterDefinition, ParameterKind, ParameterValues,
    PeakFitRequest, PowerLawModel, QuadraticModel, fit, fit_peaks,
};

#[test]
fn bin_integrated_gaussian_preserves_spectrix_area_semantics() {
    let x = (0..401)
        .map(|index| -20.0 + index as f64 * 0.1)
        .collect::<Vec<_>>();
    let model = GaussianModel::new("g0_", 12.0, 0.13, 0.7).with_bin_integration(0.1);
    let mut values = ParameterValues::new();
    values.insert("g0_amplitude", 12.0);
    values.insert("g0_center", 0.13);
    values.insert("g0_sigma", 0.7);
    let mut output = vec![0.0; x.len()];
    model
        .evaluate(&x, &values, &mut output)
        .expect("integrated Gaussian");
    close(output.iter().sum(), 120.0, 1.0e-9);

    let names = vec![
        "g0_amplitude".to_owned(),
        "g0_center".to_owned(),
        "g0_sigma".to_owned(),
    ];
    let mut jacobian = vec![0.0; x.len() * names.len()];
    assert!(
        model
            .analytic_jacobian(&x, &values, &names, &mut jacobian)
            .expect("integrated analytic Jacobian")
    );
    for (column, name) in names.iter().enumerate() {
        let initial = values.get(name).expect("parameter value");
        let step = 1.0e-6 * initial.abs().max(1.0);
        let mut lower_values = values.clone();
        let mut upper_values = values.clone();
        lower_values.insert(name.clone(), initial - step);
        upper_values.insert(name.clone(), initial + step);
        let mut lower = vec![0.0; x.len()];
        let mut upper = vec![0.0; x.len()];
        model
            .evaluate(&x, &lower_values, &mut lower)
            .expect("lower curve");
        model
            .evaluate(&x, &upper_values, &mut upper)
            .expect("upper curve");
        for row in 0..x.len() {
            let numerical = (upper[row] - lower[row]) / (2.0 * step);
            close(jacobian[row * names.len() + column], numerical, 2.0e-6);
        }
    }
}

#[test]
fn poisson_deviance_accepts_zero_count_bins_and_reports_likelihood_statistics() {
    let x = (0..8).map(|index| index as f64).collect::<Vec<_>>();
    let y = vec![0.0, 1.0, 0.0, 2.0, 1.0, 0.0, 2.0, 0.0];
    let model = ConstantModel::new("bg_", [1.0]);
    let options = FitOptions {
        objective: ObjectiveKind::PoissonDeviance,
        ..FitOptions::default()
    };
    let result =
        fit(&FitProblem::new(Box::new(model), x, y), &options).expect("Poisson constant fit");
    assert!(result.termination.success);
    assert!(result.statistics.deviance.is_some_and(f64::is_finite));
    assert!(result.statistics.bic.is_some_and(f64::is_finite));
    assert!(result.covariance.is_some());
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "actual={actual:.17e}, expected={expected:.17e}"
    );
}

type BackgroundCase = (
    Box<dyn spectrix_fitting::Model>,
    &'static [(&'static str, f64)],
    [f64; 3],
);

#[test]
fn gaussian_equation_derivatives_and_derived_values_are_consistent() {
    let model = GaussianModel::new("g0_", 12.0, 1.2, 0.7).with_bin_width(0.25);
    let mut values = ParameterValues::new();
    values.insert("g0_amplitude", 12.0);
    values.insert("g0_center", 1.2);
    values.insert("g0_sigma", 0.7);
    let x = [-0.4, 0.5, 1.2, 2.1];
    let mut curve = [0.0; 4];
    model.evaluate(&x, &values, &mut curve).expect("evaluation");
    close(
        curve[2],
        12.0 / ((2.0 * std::f64::consts::PI).sqrt() * 0.7),
        1.0e-14,
    );

    let names = vec![
        "g0_amplitude".to_owned(),
        "g0_center".to_owned(),
        "g0_sigma".to_owned(),
    ];
    let mut analytic = vec![0.0; x.len() * names.len()];
    assert!(
        model
            .analytic_jacobian(&x, &values, &names, &mut analytic)
            .expect("Jacobian")
    );
    for (column, name) in names.iter().enumerate() {
        let original = values.require(name).expect("parameter");
        let step = 1.0e-6 * original.abs().max(1.0);
        let mut plus = values.clone();
        let mut minus = values.clone();
        plus.insert(name.clone(), original + step);
        minus.insert(name.clone(), original - step);
        let mut plus_curve = [0.0; 4];
        let mut minus_curve = [0.0; 4];
        model.evaluate(&x, &plus, &mut plus_curve).expect("plus");
        model.evaluate(&x, &minus, &mut minus_curve).expect("minus");
        for row in 0..x.len() {
            let numerical = (plus_curve[row] - minus_curve[row]) / (2.0 * step);
            close(analytic[row * names.len() + column], numerical, 2.0e-9);
        }
    }

    let derived = model.derived_parameters(&values).expect("derived");
    close(
        derived
            .iter()
            .find(|value| value.name == "g0_fwhm")
            .expect("fwhm")
            .value,
        2.35482 * 0.7,
        1.0e-14,
    );
    close(
        derived
            .iter()
            .find(|value| value.name == "g0_area")
            .expect("area")
            .value,
        48.0,
        1.0e-14,
    );
}

#[test]
fn every_v1_background_model_evaluates_its_documented_equation() {
    let x = [1.0, 2.0, 4.0];
    let cases: Vec<BackgroundCase> = vec![
        (
            Box::new(ConstantModel::new("bg_", [3.5])),
            &[("bg_c", 3.5)],
            [3.5, 3.5, 3.5],
        ),
        (
            Box::new(LinearModel::new("bg_", [2.0, -1.0])),
            &[("bg_slope", 2.0), ("bg_intercept", -1.0)],
            [1.0, 3.0, 7.0],
        ),
        (
            Box::new(QuadraticModel::new("bg_", [0.5, 2.0, -1.0])),
            &[("bg_a", 0.5), ("bg_b", 2.0), ("bg_c", -1.0)],
            [1.5, 5.0, 15.0],
        ),
        (
            Box::new(ExponentialModel::new("bg_", [8.0, 2.0])),
            &[("bg_amplitude", 8.0), ("bg_decay", 2.0)],
            [
                8.0 * (-0.5_f64).exp(),
                8.0 * (-1.0_f64).exp(),
                8.0 * (-2.0_f64).exp(),
            ],
        ),
        (
            Box::new(PowerLawModel::new("bg_", [3.0, -2.0])),
            &[("bg_amplitude", 3.0), ("bg_exponent", -2.0)],
            [3.0, 0.75, 0.1875],
        ),
    ];

    for (model, parameters, expected) in cases {
        let mut values = ParameterValues::new();
        for (name, value) in parameters {
            values.insert(*name, *value);
        }
        let mut actual = [0.0; 3];
        model
            .evaluate(&x, &values, &mut actual)
            .expect("background evaluation");
        for (actual, expected) in actual.into_iter().zip(expected) {
            close(actual, expected, 1.0e-14);
        }
    }
}

#[test]
fn shared_parameter_is_classified_and_receives_propagated_error() {
    let first = GaussianModel::new("g0_", 30.0, -0.8, 0.4);
    let second = GaussianModel::new("g1_", 20.0, 0.9, 0.4).with_parameters(
        ParameterDefinition::varying("g1_amplitude", 20.0).with_bounds(Bounds::lower_bounded(0.0)),
        ParameterDefinition::varying("g1_center", 0.9),
        ParameterDefinition::fixed("g1_sigma", 0.4).equal_to("g0_sigma"),
    );
    let model = CompositeModel::new(vec![
        ModelComponent::new("g0_", Box::new(first)),
        ModelComponent::new("g1_", Box::new(second)),
    ])
    .expect("unique composite");
    let x = (-50..=50)
        .map(|index| index as f64 * 0.05)
        .collect::<Vec<_>>();
    let mut values = ParameterValues::new();
    for (name, value) in [
        ("g0_amplitude", 30.0),
        ("g0_center", -0.8),
        ("g0_sigma", 0.4),
        ("g1_amplitude", 20.0),
        ("g1_center", 0.9),
        ("g1_sigma", 0.4),
    ] {
        values.insert(name, value);
    }
    let mut y = vec![0.0; x.len()];
    model
        .evaluate(&x, &values, &mut y)
        .expect("synthetic curve");
    for (index, value) in y.iter_mut().enumerate() {
        *value += 0.002 * (index as f64).sin();
    }
    let result = fit(
        &FitProblem::new(Box::new(model), x, y),
        &FitOptions::default(),
    )
    .expect("shared fit");
    let shared = result
        .parameters
        .iter()
        .find(|parameter| parameter.name == "g1_sigma")
        .expect("shared sigma");
    assert_eq!(shared.kind, ParameterKind::Shared);
    assert!(shared.standard_error.is_some_and(|error| error > 0.0));
}

#[test]
fn singular_covariance_is_unavailable() {
    let model = LinearModel::new("", [0.0, 1.0]);
    let result = fit(
        &FitProblem::new(Box::new(model), vec![1.0; 6], vec![2.0; 6]),
        &FitOptions::default(),
    )
    .expect("usable singular estimate");
    assert!(result.covariance.is_none());
    assert!(result.confidence_band.is_none());
    assert!(
        result
            .parameters
            .iter()
            .filter(|parameter| parameter.kind == ParameterKind::Free)
            .all(|parameter| parameter.standard_error.is_none())
    );
}

#[test]
fn all_fixed_fit_matches_lmfit_unavailable_covariance_classification() {
    let model =
        ConstantModel::new("", [2.0]).with_parameters([ParameterDefinition::fixed("c", 2.0)]);
    let result = fit(
        &FitProblem::new(Box::new(model), vec![0.0, 1.0, 2.0], vec![2.1, 1.9, 2.0]),
        &FitOptions::default(),
    )
    .expect("all-fixed estimates remain usable");
    assert!(!result.termination.success);
    assert_eq!(result.termination.reason, "no_variables");
    assert!(result.covariance.is_none());
    assert!(result.confidence_band.is_none());
    assert!(
        result
            .parameters
            .iter()
            .all(|parameter| parameter.standard_error.is_none())
    );
}

#[test]
fn active_lower_bound_matches_lmfit_classification() {
    let model = ConstantModel::new("", [0.0])
        .with_parameters([
            ParameterDefinition::varying("c", 0.0).with_bounds(Bounds::lower_bounded(0.0))
        ]);
    let result = fit(
        &FitProblem::new(
            Box::new(model),
            (0..8).map(f64::from).collect(),
            vec![-2.0; 8],
        ),
        &FitOptions::default(),
    )
    .expect("active-bound estimate");
    let parameter = &result.parameters[0];
    assert!(result.termination.success);
    assert_eq!(parameter.value, 0.0);
    assert!(parameter.active_bound);
    assert_eq!(parameter.standard_error, Some(0.0));
    assert_eq!(
        result.covariance.expect("zero covariance").matrix,
        vec![vec![0.0]]
    );
}

#[test]
fn invalid_inputs_return_typed_errors() {
    let model = ConstantModel::new("", [0.0]);
    assert!(matches!(
        fit(
            &FitProblem::new(Box::new(model), vec![0.0], vec![]),
            &FitOptions::default()
        ),
        Err(FitError::LengthMismatch { .. })
    ));

    let bad_sigma = GaussianModel::new("", 1.0, 0.0, 0.0);
    assert!(matches!(
        fit(
            &FitProblem::new(Box::new(bad_sigma), vec![-1.0, 0.0, 1.0, 2.0], vec![0.0; 4],),
            &FitOptions::default()
        ),
        Err(FitError::Domain { .. })
    ));

    let request = PeakFitRequest {
        x: vec![-1.0, 1.0, 2.0, 3.0],
        y: vec![1.0; 4],
        bin_width: 1.0,
        region: [-1.0, 3.0],
        peak_seeds: vec![ManualPeakSeed {
            center: 1.0,
            sigma: 0.5,
            amplitude: 1.0,
        }],
        peak_bounds: None,
        background_markers: vec![(1.0, 3.0)],
        background: BackgroundKind::PowerLaw,
        background_seed: None,
        background_coupling: BackgroundCoupling::PrefitFrozen,
        equal_sigma: true,
        free_centers: true,
        sigma_bounds: None,
    };
    assert!(matches!(
        fit_peaks(&request, &FitOptions::default()),
        Err(FitError::Domain { .. })
    ));
}

#[test]
fn malformed_spectrum_requests_never_panic() {
    let mut state = 0x8a5c_d789_635d_2dff_u64;
    for case in 0..256 {
        let mut next = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 32) as f64 / u32::MAX as f64
        };
        let x_len = (next() * 9.0) as usize;
        let y_len = if case % 3 == 0 {
            (next() * 9.0) as usize
        } else {
            x_len
        };
        let mut x = (0..x_len)
            .map(|index| index as f64 + next())
            .collect::<Vec<_>>();
        let mut y = (0..y_len).map(|_| next() * 20.0).collect::<Vec<_>>();
        if case % 11 == 0 && !x.is_empty() {
            x[0] = f64::NAN;
        }
        if case % 13 == 0 && !y.is_empty() {
            y[0] = f64::INFINITY;
        }
        let request = PeakFitRequest {
            x,
            y,
            bin_width: if case % 7 == 0 { -1.0 } else { next() + 0.01 },
            region: [next() * 10.0, next() * 10.0],
            peak_seeds: vec![
                ManualPeakSeed {
                    center: next() * 10.0,
                    sigma: next(),
                    amplitude: next(),
                },
                ManualPeakSeed {
                    center: f64::NAN,
                    sigma: next(),
                    amplitude: next(),
                },
            ],
            peak_bounds: None,
            background_markers: vec![(next() * 5.0, next() * 5.0)],
            background: if case % 5 == 0 {
                BackgroundKind::PowerLaw
            } else {
                BackgroundKind::Linear
            },
            background_seed: None,
            background_coupling: BackgroundCoupling::PrefitFrozen,
            equal_sigma: case % 2 == 0,
            free_centers: case % 4 != 0,
            sigma_bounds: None,
        };
        let outcome = std::panic::catch_unwind(|| fit_peaks(&request, &FitOptions::default()));
        assert!(outcome.is_ok(), "malformed case {case} panicked");
    }
}

#[test]
fn joint_background_covariance_contains_background_variables() {
    let x = (0..=100)
        .map(|index| index as f64 * 0.1 + 0.1)
        .collect::<Vec<_>>();
    let y = x
        .iter()
        .enumerate()
        .map(|(index, independent)| {
            0.4 * independent
                + 1.5
                + 60.0 / ((2.0 * std::f64::consts::PI).sqrt() * 0.35)
                    * (-0.5 * ((*independent - 5.0) / 0.35).powi(2)).exp()
                + 0.01 * (index as f64).sin()
        })
        .collect::<Vec<_>>();
    let result = fit_peaks(
        &PeakFitRequest {
            x,
            y,
            bin_width: 0.1,
            region: [2.0, 8.0],
            peak_seeds: vec![ManualPeakSeed {
                center: 5.0,
                sigma: 0.4,
                amplitude: 55.0,
            }],
            peak_bounds: None,
            background_markers: vec![(2.0, 3.0), (7.0, 8.0)],
            background: BackgroundKind::Linear,
            background_seed: None,
            background_coupling: BackgroundCoupling::PrefitJoint,
            equal_sigma: true,
            free_centers: true,
            sigma_bounds: None,
        },
        &FitOptions::default(),
    )
    .expect("joint fit");
    let names = &result.fit.covariance.expect("covariance").parameter_names;
    assert!(names.iter().any(|name| name == "bg_scaled_slope"));
    assert!(names.iter().any(|name| name == "bg_scaled_level"));
    assert!(names.iter().any(|name| name == "g0_height"));
}

#[cfg(feature = "serde")]
#[test]
fn native_result_round_trips_through_json() {
    let model = LinearModel::new("", [2.0, -1.0]);
    let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
    let y = vec![-0.99, 1.02, 2.98, 5.01, 7.0];
    let result = fit(
        &FitProblem::new(Box::new(model), x, y),
        &FitOptions::default(),
    )
    .expect("linear fit");
    let json = serde_json::to_string(&result).expect("serialize");
    let decoded: spectrix_fitting::FitResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded.termination, result.termination);
    assert_eq!(decoded.parameters.len(), result.parameters.len());
    for (decoded, original) in decoded.parameters.iter().zip(&result.parameters) {
        assert_eq!(decoded.name, original.name);
        close(decoded.value, original.value, 1.0e-14);
    }
    close(
        decoded.statistics.chi_square,
        result.statistics.chi_square,
        1.0e-14,
    );
    assert_eq!(decoded.evaluation_x, result.evaluation_x);
}
