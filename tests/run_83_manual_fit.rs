//! Manual-fit end-to-end regression for the representative Xavg spectrum.

use polars::prelude::*;
use spectrix_fitting::{
    BackgroundCoupling, BackgroundKind, BackgroundSeed, FitOptions, ManualSeedEstimateRequest,
    ObjectiveKind, ParameterDefinition, PeakFitRequest, SpectrumFitResult,
    estimate_manual_peak_seeds, fit_peaks,
};

fn observation_prediction(result: &SpectrumFitResult, observations: &[f64]) -> Vec<f64> {
    observations
        .iter()
        .zip(&result.fit.raw_residuals)
        .map(|(observed, residual)| observed - residual)
        .collect()
}

fn poisson_deviance(observations: &[f64], prediction: &[f64]) -> f64 {
    observations
        .iter()
        .zip(prediction)
        .map(|(observed, predicted)| {
            let mean = predicted.max(1.0e-12);
            if *observed == 0.0 {
                2.0 * mean
            } else {
                2.0 * (mean - observed + observed * (observed / mean).ln())
            }
        })
        .sum()
}

fn run_83_histogram() -> (Vec<f64>, Vec<f64>) {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("example")
        .join("run_83_reduced.parquet");
    let parquet = PlRefPath::try_from_pathbuf(path).expect("fixture path");
    let frame = LazyFrame::scan_parquet(parquet, Default::default())
        .expect("scan run_83 parquet")
        .select([col("Xavg")])
        .collect()
        .expect("collect Xavg");
    let values = frame
        .column("Xavg")
        .expect("Xavg column")
        .as_materialized_series()
        .f64()
        .expect("Xavg f64");
    let mut counts = vec![0.0; 600];
    for value in values.iter().flatten() {
        if (-300.0..300.0).contains(&value) {
            counts[(value + 300.0).floor() as usize] += 1.0;
        }
    }
    let centers = (0..600).map(|index| -299.5 + index as f64).collect();
    (centers, counts)
}

#[test]
fn root_parquet_extraction_matches_the_committed_600_bin_fixture() {
    let (_, actual) = run_83_histogram();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../crates/spectrix-fitting/tests/fixtures/run_83_xavg_600.json"
    ))
    .expect("valid committed histogram fixture");
    assert_eq!(fixture["bins"].as_u64(), Some(600));
    assert_eq!(fixture["column"].as_str(), Some("Xavg"));
    assert_eq!(fixture["encoding"].as_str(), Some("u16-be-hex"));
    let encoded = fixture["counts"].as_str().expect("fixture counts");
    let expected = encoded
        .as_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| {
            let digits = std::str::from_utf8(chunk).expect("ASCII hex");
            u16::from_str_radix(digits, 16).expect("u16 count") as f64
        })
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 600);
    assert_eq!(actual, expected);
}

#[test]
fn canonical_manual_fit_uses_the_ten_explicit_run_83_markers() {
    let (x, y) = run_83_histogram();
    let markers = vec![
        -205.0, -177.0, -152.0, -121.0, -65.0, -45.0, 10.0, 26.0, 50.0, 80.0,
    ];
    let background_markers = vec![(-220.0, -213.0), (93.0, 100.0)];
    let options = FitOptions::default();
    let estimate = estimate_manual_peak_seeds(
        &ManualSeedEstimateRequest {
            x: x.clone(),
            y: y.clone(),
            bin_width: 1.0,
            region: [-220.0, 100.0],
            peak_markers: markers.clone(),
            background_markers: background_markers.clone(),
            background: BackgroundKind::Constant,
            background_seed: None,
            equal_sigma: false,
        },
        &options,
    )
    .expect("run-83 manual seed estimate");
    assert!(estimate.peaks.iter().all(|peak| peak.valid));
    assert_eq!(
        estimate
            .peaks
            .iter()
            .map(|peak| peak.seed.center)
            .collect::<Vec<_>>(),
        markers
    );

    let region_y = x
        .iter()
        .zip(&y)
        .filter(|(independent, _)| **independent >= -220.0 && **independent <= 100.0)
        .map(|(_, count)| *count)
        .collect::<Vec<_>>();
    let request = PeakFitRequest {
        x,
        y: y.clone(),
        bin_width: 1.0,
        region: [-220.0, 100.0],
        peak_seeds: estimate.peaks.iter().map(|peak| peak.seed).collect(),
        peak_bounds: Some(estimate.peaks.iter().map(|peak| peak.bounds).collect()),
        background_markers,
        background: BackgroundKind::Constant,
        background_seed: None,
        background_coupling: BackgroundCoupling::PrefitJoint,
        equal_sigma: false,
        free_centers: true,
        sigma_bounds: None,
    };
    let least_squares = fit_peaks(
        &request,
        &FitOptions {
            objective: ObjectiveKind::LeastSquares,
            ..FitOptions::default()
        },
    )
    .expect("run-83 least-squares fit");
    let poisson = fit_peaks(
        &request,
        &FitOptions {
            objective: ObjectiveKind::PoissonDeviance,
            ..FitOptions::default()
        },
    )
    .expect("run-83 Poisson fit");

    assert_eq!(poisson.peak_seeds.len(), markers.len());
    assert!(poisson.fit.termination.success);
    assert!(poisson.fit.covariance.is_some());
    assert!(poisson.fit.statistics.objective_improvement.unwrap() > 0.0);
    let least_squares_prediction = observation_prediction(&least_squares, &region_y);
    let poisson_prediction = observation_prediction(&poisson, &region_y);
    let least_squares_deviance = poisson_deviance(&region_y, &least_squares_prediction);
    let fitted_deviance = poisson_deviance(&region_y, &poisson_prediction);
    assert!(
        fitted_deviance <= least_squares_deviance + 1.0e-7 * least_squares_deviance.max(1.0),
        "Poisson deviance {fitted_deviance} exceeded the LS-curve deviance {least_squares_deviance}",
    );

    for (index, marker) in markers.iter().enumerate() {
        let component = poisson
            .fit
            .components
            .iter()
            .find(|component| component.name == format!("g{index}_"))
            .expect("Gaussian component");
        let maximum_index = component
            .values
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index)
            .expect("nonempty component");
        let maximum_x = poisson.fit.evaluation_x[maximum_index];
        let fitted_center = poisson
            .fit
            .parameters
            .iter()
            .find(|parameter| parameter.name == format!("g{index}_center"))
            .expect("fitted center");
        assert!(
            !fitted_center.active_bound,
            "peak {marker} hit a center bound"
        );
        assert!(
            (maximum_x - fitted_center.value).abs() <= 1.0,
            "peak {marker}: component maximum {maximum_x} is not aligned with center {}",
            fitted_center.value,
        );
    }
}

#[test]
fn every_run_83_marker_has_a_stable_poisson_single_peak_fit() {
    let (x, y) = run_83_histogram();
    let markers = [
        -205.0, -177.0, -152.0, -121.0, -65.0, -45.0, 10.0, 26.0, 50.0, 80.0,
    ];
    for (index, marker) in markers.iter().copied().enumerate() {
        let lower = if index == 0 {
            -220.0
        } else {
            0.5 * (markers[index - 1] + marker)
        };
        let upper = if index + 1 == markers.len() {
            100.0
        } else {
            0.5 * (marker + markers[index + 1])
        };
        let background_width = 2.0_f64.min(0.2 * (upper - lower));
        let background_markers = vec![
            (lower, lower + background_width),
            (upper - background_width, upper),
        ];
        let estimate = estimate_manual_peak_seeds(
            &ManualSeedEstimateRequest {
                x: x.clone(),
                y: y.clone(),
                bin_width: 1.0,
                region: [lower, upper],
                peak_markers: vec![marker],
                background_markers: background_markers.clone(),
                background: BackgroundKind::Constant,
                background_seed: None,
                equal_sigma: false,
            },
            &FitOptions::default(),
        )
        .unwrap_or_else(|error| panic!("marker {marker}: seed estimate failed: {error}"));
        assert!(estimate.peaks[0].valid, "marker {marker}: invalid seed");
        let background_level = estimate
            .background_prefit
            .parameters
            .iter()
            .find(|parameter| parameter.name == "bg_c")
            .expect("constant background")
            .value;
        let request = PeakFitRequest {
            x: x.clone(),
            y: y.clone(),
            bin_width: 1.0,
            region: [lower, upper],
            peak_seeds: vec![estimate.peaks[0].seed],
            peak_bounds: Some(vec![estimate.peaks[0].bounds]),
            background_markers,
            background: BackgroundKind::Constant,
            background_seed: Some(BackgroundSeed {
                parameters: vec![ParameterDefinition::fixed("bg_c", background_level)],
            }),
            background_coupling: BackgroundCoupling::PrefitFrozen,
            equal_sigma: false,
            free_centers: true,
            sigma_bounds: None,
        };
        let least_squares = fit_peaks(
            &request,
            &FitOptions {
                objective: ObjectiveKind::LeastSquares,
                ..FitOptions::default()
            },
        )
        .unwrap_or_else(|error| panic!("marker {marker}: least-squares fit failed: {error}"));
        let poisson = fit_peaks(
            &request,
            &FitOptions {
                objective: ObjectiveKind::PoissonDeviance,
                ..FitOptions::default()
            },
        )
        .unwrap_or_else(|error| panic!("marker {marker}: Poisson fit failed: {error}"));
        assert!(
            poisson.fit.termination.success,
            "marker {marker}: {:?}",
            poisson.fit.termination,
        );
        assert!(
            poisson.fit.statistics.objective_improvement.unwrap() >= -1.0e-8,
            "marker {marker}: objective worsened",
        );

        let region_y = x
            .iter()
            .zip(&y)
            .filter(|(independent, _)| **independent >= lower && **independent <= upper)
            .map(|(_, count)| *count)
            .collect::<Vec<_>>();
        let least_squares_prediction = observation_prediction(&least_squares, &region_y);
        let poisson_prediction = observation_prediction(&poisson, &region_y);
        let least_squares_deviance = poisson_deviance(&region_y, &least_squares_prediction);
        let fitted_deviance = poisson_deviance(&region_y, &poisson_prediction);
        assert!(
            fitted_deviance <= least_squares_deviance + 1.0e-7 * least_squares_deviance.max(1.0),
            "marker {marker}: Poisson deviance {fitted_deviance} exceeded LS deviance {least_squares_deviance}",
        );
        let center = poisson
            .fit
            .parameters
            .iter()
            .find(|parameter| parameter.name == "g0_center")
            .expect("fitted center");
        assert!(!center.active_bound, "marker {marker}: center hit a bound");
        let observed_maximum = region_y
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index)
            .expect("region observations");
        let predicted_maximum = poisson_prediction
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index)
            .expect("region prediction");
        assert!(
            observed_maximum.abs_diff(predicted_maximum) <= 2,
            "marker {marker}: observed maximum bin {observed_maximum}, fitted maximum bin {predicted_maximum}",
        );
    }
}
