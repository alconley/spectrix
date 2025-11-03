use crate::fitter::models::gaussian::GaussianParameters;

#[expect(clippy::needless_pass_by_value)]
pub fn log_axis_spacer(input: egui_plot::GridInput) -> Vec<egui_plot::GridMark> {
    let (min, max) = input.bounds;
    let mut marks = vec![];
    for i in min.floor() as i32..=max.ceil() as i32 {
        marks.extend(
            (10..100)
                .map(|j| {
                    let value = i as f64 + (j as f64).log10() - 1.0;
                    let step_size = if j == 10 {
                        1.0
                    } else if j % 10 == 0 {
                        0.1
                    } else {
                        0.01
                    };
                    egui_plot::GridMark { value, step_size }
                })
                .filter(|gm| (min..=max).contains(&gm.value)),
        );
    }
    marks
}

pub fn log_axis_formatter(
    gm: egui_plot::GridMark,
    _bounds: &std::ops::RangeInclusive<f64>,
    max_size: usize,
) -> String {
    let min_precision = (-gm.value + 1.0).ceil().clamp(1.0, 10.0) as usize;
    let digits = (gm.value).ceil().max(1.0) as usize;
    let size = digits + min_precision + 1;
    let value = 10.0f64.powf(gm.value);
    if size < max_size {
        let precision = max_size.saturating_sub(digits + 1);
        format!("{value:.precision$}")
    } else {
        let exp_digits = (digits as f64).log10() as usize;
        let precision = max_size.saturating_sub(exp_digits).saturating_sub(3);
        format!("{value:.precision$e}")
    }
}

// Keep “nice” log rounding to 1–3–10 decades.
pub fn nice_log_ceil(x: f64) -> f64 {
    if !x.is_finite() || x <= 0.0 {
        return 1.0;
    }
    let exp = x.log10().floor();
    let base = 10f64.powf(exp);
    let mant = x / base; // in [1,10)
    let m = if mant <= 1.0 {
        1.0
    } else if mant <= 3.0 {
        3.0
    } else {
        10.0
    };
    m * base
}

pub fn nice_log_floor(x: f64) -> f64 {
    if !x.is_finite() || x <= 0.0 {
        return 1.0;
    }
    let exp = x.log10().floor();
    let base = 10f64.powf(exp);
    let mant = x / base; // in [1,10)
    let m = if mant >= 3.0 { 3.0 } else { 1.0 };
    m * base
}

/// Compute inverse-variance weighted average of calibrated means (and its uncertainty).
///
/// Falls back to unweighted mean + standard error if uncertainties are missing/invalid.
/// Returns (mean, uncertainty). Both None if no calibrated means found.
pub fn avg_calibrated_mean(
    pts: &[(
        f64,
        f64,
        f64,
        f64,
        GaussianParameters,
        egui::Color32,
        egui_plot::MarkerShape,
    )],
) -> (Option<f64>, Option<f64>) {
    // Collect (mean, sigma) pairs where calibrated mean exists
    let mut with_unc: Vec<(f64, f64)> = Vec::new(); // (m_i, σ_i)
    let mut bare: Vec<f64> = Vec::new(); // m_i when σ_i missing

    for (_, _, _, _, params, _, _) in pts {
        if let Some(m) = params.mean.calibrated_value {
            if let Some(dm) = params.mean.calibrated_uncertainty
                && dm.is_finite()
                && dm > 0.0
            {
                with_unc.push((m, dm));
                continue;
            }
            bare.push(m);
        }
    }

    if !with_unc.is_empty() {
        // Inverse-variance weighted mean
        let mut wsum = 0.0;
        let mut wmsum = 0.0;
        for (m, dm) in with_unc {
            let w = 1.0 / (dm * dm);
            wsum += w;
            wmsum += w * m;
        }
        if wsum > 0.0 {
            let mean = wmsum / wsum;
            let unc = 1.0 / wsum.sqrt();
            return (Some(mean), Some(unc));
        }
    }

    // Fallback: unweighted mean + standard error (if possible)
    let all: Vec<f64> = if !bare.is_empty() {
        bare
    } else {
        // No valid values at all
        return (None, None);
    };

    let n = all.len() as f64;
    let mean = all.iter().copied().sum::<f64>() / n;
    if all.len() >= 2 {
        let var = all.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let se = (var / n).sqrt();
        (Some(mean), Some(se))
    } else {
        (Some(mean), None)
    }
}

// NEW: average points with the same (or nearly the same) angle.
// Uses inverse-variance weights if all dy>0; else simple mean.
// Keeps the first color seen in the group.
#[inline]
pub fn average_points_by_angle(
    points: &mut [(f64, f64, f64, egui::Color32)], // (ang_deg, y, dy, color)
    tol_deg: f64,
) -> Vec<(f64, f64, f64, egui::Color32)> {
    if points.is_empty() {
        return Vec::new();
    }
    points.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64, f64, egui::Color32)> = Vec::new();

    let mut grp: Vec<(f64, f64, f64, egui::Color32)> = Vec::new();
    let mut start_ang = points[0].0;

    for &(ang, y, dy, col) in points.iter() {
        if (ang - start_ang).abs() <= tol_deg {
            grp.push((ang, y, dy, col));
        } else {
            // flush current group
            out.push(average_group(&grp));
            grp.clear();
            grp.push((ang, y, dy, col));
            start_ang = ang;
        }
    }
    // flush last group
    out.push(average_group(&grp));

    out
}

#[inline]
pub fn average_group(grp: &[(f64, f64, f64, egui::Color32)]) -> (f64, f64, f64, egui::Color32) {
    let color = grp[0].3; // keep first color
    let ang_avg = grp.iter().map(|g| g.0).sum::<f64>() / (grp.len() as f64);

    // If all dy>0 and finite → inverse-variance weighted mean
    let all_w = grp.iter().all(|g| g.2.is_finite() && g.2 > 0.0);
    if all_w {
        let mut wsum = 0.0;
        let mut wysum = 0.0;
        for (_, y, dy, _) in grp {
            let w = 1.0 / (dy * dy);
            wsum += w;
            wysum += w * y;
        }
        let ybar = wysum / wsum;
        let dybar = 1.0 / wsum.sqrt();
        (ang_avg, ybar, dybar, color)
    } else {
        // simple mean; uncertainty = standard error if n>1, else keep dy if any
        let n = grp.len() as f64;
        let ybar = grp.iter().map(|g| g.1).sum::<f64>() / n;
        if grp.len() > 1 {
            let var = grp.iter().map(|g| (g.1 - ybar).powi(2)).sum::<f64>() / (n - 1.0);
            let se = (var / n).sqrt();
            (ang_avg, ybar, se, color)
        } else {
            (ang_avg, ybar, grp[0].2, color)
        }
    }
}
