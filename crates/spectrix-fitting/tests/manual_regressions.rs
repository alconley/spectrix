use spectrix_fitting::{
    BackgroundCoupling, BackgroundKind, FitOptions, ManualPeakSeed, ObjectiveKind, PeakFitRequest,
    fit_peaks,
};

#[test]
fn manual_peak_fit_handles_overlap_zero_bins_and_reversed_regions() {
    let x = (0..161)
        .map(|index| -8.0 + index as f64 * 0.1)
        .collect::<Vec<_>>();
    let y = x
        .iter()
        .enumerate()
        .map(|(index, value)| {
            if index % 37 == 0 {
                0.0
            } else {
                2.0 + 70.0 / (2.506_628_274_631 * 0.42)
                    * (-0.5 * ((*value + 0.45) / 0.42).powi(2)).exp()
                    + 55.0 / (2.506_628_274_631 * 0.75)
                        * (-0.5 * ((*value - 0.55) / 0.75).powi(2)).exp()
            }
        })
        .collect::<Vec<_>>();
    let options = FitOptions {
        objective: ObjectiveKind::PoissonDeviance,
        ..FitOptions::default()
    };
    let result = fit_peaks(
        &PeakFitRequest {
            x,
            y,
            bin_width: 0.1,
            region: [3.0, -3.0],
            peak_seeds: vec![
                ManualPeakSeed {
                    center: 0.6,
                    sigma: 0.7,
                    amplitude: 55.0,
                },
                ManualPeakSeed {
                    center: -0.5,
                    sigma: 0.4,
                    amplitude: 70.0,
                },
            ],
            peak_bounds: None,
            background_markers: vec![(-3.0, -2.2), (2.2, 3.0)],
            background: BackgroundKind::Constant,
            background_seed: None,
            background_coupling: BackgroundCoupling::PrefitJoint,
            equal_sigma: false,
            free_centers: true,
            sigma_bounds: None,
        },
        &options,
    )
    .expect("manual overlapping-peak fit");
    assert_eq!(result.region, [-3.0, 3.0]);
    assert_eq!(result.peak_markers, vec![-0.5, 0.6]);
    let centers = result
        .fit
        .parameters
        .iter()
        .filter(|parameter| parameter.name.ends_with("_center"))
        .map(|parameter| parameter.value)
        .collect::<Vec<_>>();
    assert!(centers.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(result.fit.statistics.deviance.is_some());
}

#[test]
fn invalid_authoritative_markers_return_actionable_errors() {
    let request = PeakFitRequest {
        x: vec![0.5, 1.5, 2.5, 3.5],
        y: vec![1.0, 4.0, 2.0, 1.0],
        bin_width: 1.0,
        region: [0.0, 4.0],
        peak_seeds: vec![
            ManualPeakSeed {
                center: 1.4,
                sigma: 0.5,
                amplitude: 2.0,
            },
            ManualPeakSeed {
                center: 1.8,
                sigma: 0.5,
                amplitude: 2.0,
            },
        ],
        peak_bounds: None,
        background_markers: Vec::new(),
        background: BackgroundKind::None,
        background_seed: None,
        background_coupling: BackgroundCoupling::PrefitJoint,
        equal_sigma: true,
        free_centers: true,
        sigma_bounds: None,
    };
    let error = fit_peaks(&request, &FitOptions::default()).expect_err("duplicate markers");
    assert!(error.to_string().contains("duplicate within one bin"));
}
