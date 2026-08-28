use std::{collections::BTreeMap, hint::black_box, time::Instant};

use spectrix_fitting::{
    BackgroundCoupling, BackgroundKind, FitOptions, ManualPeakSeed, ObjectiveKind, PeakFitRequest,
    fit_peaks,
};

fn request(bin_count: usize, peak_count: usize) -> PeakFitRequest {
    let bin_width = 10.0 / (bin_count - 1) as f64;
    let x = (0..bin_count)
        .map(|index| 1.0 + index as f64 * bin_width)
        .collect::<Vec<_>>();
    let true_centers = (0..peak_count)
        .map(|index| 2.0 + 8.0 * (index + 1) as f64 / (peak_count + 1) as f64)
        .collect::<Vec<_>>();
    let peak_seeds = true_centers
        .iter()
        .enumerate()
        .map(|(peak, &center)| ManualPeakSeed {
            center,
            sigma: 0.11 + peak as f64 * 0.006,
            amplitude: 75.0 + peak as f64 * 8.0,
        })
        .collect();
    let y = x
        .iter()
        .enumerate()
        .map(|(index, independent)| {
            let peaks = true_centers
                .iter()
                .enumerate()
                .map(|(peak, center)| {
                    let amplitude = 75.0 + peak as f64 * 8.0;
                    let sigma = 0.11 + peak as f64 * 0.006;
                    amplitude / ((2.0 * std::f64::consts::PI).sqrt() * sigma)
                        * (-0.5 * ((*independent - center) / sigma).powi(2)).exp()
                })
                .sum::<f64>();
            3.0 + 0.18 * independent + peaks + 0.015 * (index as f64 * 0.17).sin()
        })
        .collect();

    PeakFitRequest {
        x,
        y,
        bin_width,
        region: [1.0, 11.0],
        peak_seeds,
        peak_bounds: None,
        background_markers: vec![(1.0, 1.6), (10.4, 11.0)],
        background: BackgroundKind::Linear,
        background_seed: None,
        background_coupling: BackgroundCoupling::PrefitFrozen,
        equal_sigma: false,
        free_centers: false,
        sigma_bounds: None,
    }
}

fn canonical_request() -> PeakFitRequest {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/run_83_xavg_600.json"))
            .expect("canonical fixture");
    let encoded = fixture["counts"].as_str().expect("encoded counts");
    let y = encoded
        .as_bytes()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| {
            let digits = std::str::from_utf8(chunk).expect("ASCII hex");
            u16::from_str_radix(digits, 16).expect("u16 count") as f64
        })
        .collect();
    PeakFitRequest {
        x: (0..600).map(|index| -299.5 + index as f64).collect(),
        y,
        bin_width: 1.0,
        region: [-220.0, 100.0],
        peak_seeds: [
            -205.0, -177.0, -152.0, -121.0, -65.0, -45.0, 10.0, 26.0, 50.0, 80.0,
        ]
        .into_iter()
        .map(|center| ManualPeakSeed {
            center,
            sigma: 4.0,
            amplitude: 1_000.0,
        })
        .collect(),
        peak_bounds: None,
        background_markers: vec![(-220.0, -212.0), (92.0, 100.0)],
        background: BackgroundKind::Constant,
        background_seed: None,
        background_coupling: BackgroundCoupling::PrefitJoint,
        equal_sigma: false,
        free_centers: true,
        sigma_bounds: None,
    }
}

fn main() {
    let mut timings = BTreeMap::new();
    for peak_count in [1, 3, 8] {
        for bin_count in [512, 2_048, 8_192] {
            let request = request(bin_count, peak_count);
            let options = FitOptions::default();
            black_box(fit_peaks(&request, &options).expect("benchmark warmup must fit"));
            let repeats = match bin_count {
                512 => 5,
                2_048 => 3,
                _ => 1,
            };
            let start = Instant::now();
            for _ in 0..repeats {
                black_box(fit_peaks(&request, &options).expect("benchmark case must fit"));
            }
            let microseconds = start.elapsed().as_secs_f64() * 1.0e6 / repeats as f64;
            timings.insert(format!("{peak_count}p_{bin_count}b"), microseconds);
        }
    }
    let canonical = canonical_request();
    let poisson = FitOptions {
        objective: ObjectiveKind::PoissonDeviance,
        ..FitOptions::default()
    };
    black_box(fit_peaks(&canonical, &poisson).expect("canonical warmup must fit"));
    let start = Instant::now();
    black_box(fit_peaks(&canonical, &poisson).expect("canonical benchmark must fit"));
    timings.insert(
        "canonical_manual_600b".to_owned(),
        start.elapsed().as_secs_f64() * 1.0e6,
    );
    println!(
        "{}",
        serde_json::to_string(&timings).expect("serializing benchmark output cannot fail")
    );
}
