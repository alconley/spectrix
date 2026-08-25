use std::cmp::Ordering;

use crate::{
    Bound, Bounds, CompositeModel, ConstantModel, ExponentialModel, FitError, FitOptions,
    FitProblem, FitResult, GaussianModel, LinearModel, Model, ModelComponent, ParameterDefinition,
    ParameterEstimate, PowerLawModel, QuadraticModel, fit,
};

/// How a background prefit is coupled to the subsequent peak fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum BackgroundCoupling {
    /// Fit the background first and freeze it during the peak fit.
    #[default]
    PrefitFrozen,
    /// Fit the background first and then vary enabled background parameters jointly with peaks.
    PrefitJoint,
}

/// Background equations available in version 0.1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum BackgroundKind {
    /// A fixed zero background.
    #[default]
    None,
    /// A varying constant background.
    Constant,
    /// `slope * x + intercept`.
    Linear,
    /// `a * x^2 + b * x + c`.
    Quadratic,
    /// `amplitude * exp(-x / decay)`.
    Exponential,
    /// `amplitude * x^exponent`.
    PowerLaw,
}

/// Optional initial background parameter definitions.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BackgroundSeed {
    /// Definitions keyed with lmfit-style `bg_` names (for example `bg_slope`).
    pub parameters: Vec<ParameterDefinition>,
}

/// Per-peak or shared sigma constraints.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SigmaBounds {
    /// Lower bounds; one entry for equal sigma or one per peak otherwise.
    pub minima: Vec<f64>,
    /// Upper bounds; one entry for equal sigma or one per peak otherwise.
    pub maxima: Vec<f64>,
}

/// Request for fitting only a spectrum background.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BackgroundFitRequest {
    /// Bin centers or independent values.
    pub x: Vec<f64>,
    /// Counts or dependent values.
    pub y: Vec<f64>,
    /// Positive bin width.
    pub bin_width: f64,
    /// Inclusive region; marker order is normalized.
    pub region: [f64; 2],
    /// Inclusive background marker windows. Empty uses the bins nearest both
    /// region edges, expanding the automatic sample when the model needs more
    /// observations than the first edge bins provide.
    pub markers: Vec<(f64, f64)>,
    /// Background equation.
    pub kind: BackgroundKind,
    /// Optional bounds, initial values, and fixed/varying settings.
    pub seed: Option<BackgroundSeed>,
}

/// Request for Gaussian peak fitting with an optional background.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PeakFitRequest {
    /// Bin centers.
    pub x: Vec<f64>,
    /// Counts.
    pub y: Vec<f64>,
    /// Positive bin width. Spectrix area is amplitude divided by this value.
    pub bin_width: f64,
    /// Inclusive fit region; marker order is normalized.
    pub region: [f64; 2],
    /// Peak markers. Empty selects the strongest region bin.
    pub peak_markers: Vec<f64>,
    /// Inclusive background marker windows. Empty uses the bins nearest both
    /// region edges, expanding the automatic sample when the model needs more
    /// observations than the first edge bins provide.
    pub background_markers: Vec<(f64, f64)>,
    /// Background equation.
    pub background: BackgroundKind,
    /// Optional background initial values and constraints.
    pub background_seed: Option<BackgroundSeed>,
    /// Background/peak covariance behavior.
    pub background_coupling: BackgroundCoupling,
    /// Whether every Gaussian shares `g0_sigma`.
    pub equal_sigma: bool,
    /// Whether Gaussian centers may vary.
    pub free_centers: bool,
    /// Optional sigma constraints.
    pub sigma_bounds: Option<SigmaBounds>,
}

/// Peak fit plus its independently useful background prefit.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SpectrumFitResult {
    /// Composite Gaussian/background fit.
    pub fit: FitResult,
    /// Background-only prefit with its own covariance and confidence band.
    pub background_prefit: FitResult,
    /// Sorted inclusive region used by the fit.
    pub region: [f64; 2],
    /// Sorted, in-region peak markers used for initialization.
    pub peak_markers: Vec<f64>,
    /// Background coupling used by the composite fit.
    pub background_coupling: BackgroundCoupling,
}

/// Fits a background using explicit marker windows or an adaptive region-edge fallback.
pub fn fit_background(
    request: &BackgroundFitRequest,
    options: &FitOptions,
) -> Result<FitResult, FitError> {
    validate_data(&request.x, &request.y, request.bin_width)?;
    let region = sorted_region(request.region)?;
    validate_power_law_domain(request.kind, &request.x)?;
    let model = background_model(request.kind, request.seed.as_ref(), None, false);
    let minimum_observations = model
        .parameter_definitions()
        .iter()
        .filter(|parameter| parameter.vary && parameter.binding.is_none())
        .count()
        .saturating_add(1);
    let (x, y) = background_data(
        &request.x,
        &request.y,
        region,
        request.bin_width,
        &request.markers,
        minimum_observations,
    )?;
    let mut background_options = options.clone();
    if background_options.evaluation_x.is_none() {
        background_options.evaluation_x = Some(linspace(region[0], region[1], 256)?);
    }
    fit(&FitProblem::new(model, x, y), &background_options)
}

/// Fits Gaussian peaks after applying Spectrix-compatible preprocessing.
#[expect(
    clippy::too_many_lines,
    reason = "Spectrix-compatible preprocessing is kept together for reviewability"
)]
pub fn fit_peaks(
    request: &PeakFitRequest,
    options: &FitOptions,
) -> Result<SpectrumFitResult, FitError> {
    validate_data(&request.x, &request.y, request.bin_width)?;
    let region = sorted_region(request.region)?;
    validate_power_law_domain(request.background, &request.x)?;
    let (region_x, region_y) = region_data(&request.x, &request.y, region)?;
    if region_x.len() < 2 {
        return Err(FitError::InvalidRegion);
    }

    let mut peak_markers = request
        .peak_markers
        .iter()
        .copied()
        .filter(|marker| marker.is_finite() && *marker >= region[0] && *marker <= region[1])
        .collect::<Vec<_>>();
    peak_markers.sort_by(f64::total_cmp);
    if peak_markers.is_empty() {
        let strongest = region_y
            .iter()
            .enumerate()
            .max_by(|(_, left), (_, right)| left.total_cmp(right))
            .map(|(index, _)| region_x[index])
            .ok_or(FitError::InvalidRegion)?;
        peak_markers.push(strongest);
    }

    let background_request = BackgroundFitRequest {
        x: request.x.clone(),
        y: request.y.clone(),
        bin_width: request.bin_width,
        region,
        markers: request.background_markers.clone(),
        kind: request.background,
        seed: request.background_seed.clone(),
    };
    let background_prefit = fit_background(&background_request, options)?;

    let background_values = background_prefit
        .parameters
        .iter()
        .filter(|parameter| parameter.name.starts_with("bg_"))
        .cloned()
        .collect::<Vec<_>>();
    let background_at_peaks =
        evaluate_background_estimates(request.background, &background_values, &peak_markers)?;

    let peak_heights = peak_markers
        .iter()
        .map(|marker| region_y[nearest_index(&region_x, *marker)])
        .collect::<Vec<_>>();
    let strongest_index = peak_heights
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map_or(0, |(index, _)| index);
    let per_peak_sigma = peak_markers
        .iter()
        .map(|marker| estimate_sigma(&region_x, &region_y, *marker, request.bin_width))
        .collect::<Vec<_>>();
    let shared_sigma = per_peak_sigma[strongest_index];
    let sigma_bounds = normalized_sigma_bounds(
        request.sigma_bounds.as_ref(),
        request.equal_sigma,
        peak_markers.len(),
    )?;

    let background_joint = request.background_coupling == BackgroundCoupling::PrefitJoint;
    let background = background_model(
        request.background,
        request.background_seed.as_ref(),
        Some(&background_values),
        background_joint,
    );
    let mut composite = CompositeModel::default();
    composite.push(ModelComponent::new("background", background))?;

    for (index, marker) in peak_markers.iter().copied().enumerate() {
        let prefix = format!("g{index}_");
        let amplitude_name = format!("{prefix}amplitude");
        let center_name = format!("{prefix}center");
        let sigma_name = format!("{prefix}sigma");
        let adjusted_height = peak_heights[index] - background_at_peaks[index];
        let amplitude_initial = (adjusted_height * shared_sigma / 0.398_942_3).max(0.0);
        let amplitude = ParameterDefinition::varying(amplitude_name, amplitude_initial)
            .with_bounds(Bounds::lower_bounded(0.0));
        let center_bounds = center_bounds(index, &peak_markers, region, shared_sigma);
        let mut center =
            ParameterDefinition::varying(center_name, marker).with_bounds(center_bounds);
        center.vary = request.free_centers;

        let automatic_sigma = if request.equal_sigma {
            shared_sigma
        } else {
            per_peak_sigma[index]
        };
        let sigma_interval = sigma_bounds.as_ref().map_or_else(
            || Bounds::lower_bounded(0.0),
            |(minima, maxima)| {
                let constraint_index = if request.equal_sigma { 0 } else { index };
                Bounds::finite(minima[constraint_index], maxima[constraint_index])
            },
        );
        let sigma_initial = clamp_to_bounds(automatic_sigma, sigma_interval);
        let sigma = if request.equal_sigma && index > 0 {
            ParameterDefinition::fixed(sigma_name, sigma_initial).equal_to("g0_sigma")
        } else {
            ParameterDefinition::varying(sigma_name, sigma_initial).with_bounds(sigma_interval)
        };
        let gaussian = GaussianModel::new(prefix.clone(), amplitude_initial, marker, sigma_initial)
            .with_parameters(amplitude, center, sigma)
            .with_bin_width(request.bin_width);
        composite.push(ModelComponent::new(prefix, Box::new(gaussian)))?;
    }

    let evaluation_count = region_x
        .len()
        .checked_mul(50)
        .ok_or(FitError::AllocationLimit {
            requested: usize::MAX,
            limit: 16_777_216,
        })?;
    let mut peak_options = options.clone();
    if peak_options.evaluation_x.is_none() {
        peak_options.evaluation_x = Some(linspace(
            region_x[0],
            region_x[region_x.len() - 1],
            evaluation_count,
        )?);
    }
    let fit = fit(
        &FitProblem::new(Box::new(composite), region_x, region_y),
        &peak_options,
    )?;

    Ok(SpectrumFitResult {
        fit,
        background_prefit,
        region,
        peak_markers,
        background_coupling: request.background_coupling,
    })
}

fn background_model(
    kind: BackgroundKind,
    seed: Option<&BackgroundSeed>,
    estimates: Option<&[ParameterEstimate]>,
    refine: bool,
) -> Box<dyn Model> {
    let defaults = default_background_parameters(kind);
    let definitions = defaults
        .into_iter()
        .map(|default| {
            let seeded = seed
                .and_then(|all| find_definition(&all.parameters, &default.name))
                .cloned()
                .unwrap_or(default);
            let estimate = estimates.and_then(|all| find_estimate(all, &seeded.name));
            let mut definition = seeded;
            if let Some(estimate) = estimate {
                definition.initial = clamp_to_bounds(estimate.value, definition.bounds);
                definition.vary &= refine;
            }
            definition
        })
        .collect::<Vec<_>>();
    match kind {
        BackgroundKind::None | BackgroundKind::Constant => Box::new(
            ConstantModel::new("bg_", [definitions[0].initial])
                .with_parameters([definitions[0].clone()]),
        ),
        BackgroundKind::Linear => Box::new(
            LinearModel::new("bg_", [definitions[0].initial, definitions[1].initial])
                .with_parameters([definitions[0].clone(), definitions[1].clone()]),
        ),
        BackgroundKind::Quadratic => Box::new(
            QuadraticModel::new(
                "bg_",
                [
                    definitions[0].initial,
                    definitions[1].initial,
                    definitions[2].initial,
                ],
            )
            .with_parameters([
                definitions[0].clone(),
                definitions[1].clone(),
                definitions[2].clone(),
            ]),
        ),
        BackgroundKind::Exponential => Box::new(
            ExponentialModel::new("bg_", [definitions[0].initial, definitions[1].initial])
                .with_parameters([definitions[0].clone(), definitions[1].clone()]),
        ),
        BackgroundKind::PowerLaw => Box::new(
            PowerLawModel::new("bg_", [definitions[0].initial, definitions[1].initial])
                .with_parameters([definitions[0].clone(), definitions[1].clone()]),
        ),
    }
}

fn default_background_parameters(kind: BackgroundKind) -> Vec<ParameterDefinition> {
    match kind {
        BackgroundKind::None => vec![ParameterDefinition::fixed("bg_c", 0.0)],
        BackgroundKind::Constant => vec![ParameterDefinition::varying("bg_c", 0.0)],
        BackgroundKind::Linear => vec![
            ParameterDefinition::varying("bg_slope", 0.0),
            ParameterDefinition::varying("bg_intercept", 0.0),
        ],
        BackgroundKind::Quadratic => vec![
            ParameterDefinition::varying("bg_a", 0.0),
            ParameterDefinition::varying("bg_b", 0.0),
            ParameterDefinition::varying("bg_c", 0.0),
        ],
        BackgroundKind::Exponential => vec![
            ParameterDefinition::varying("bg_amplitude", 0.0),
            ParameterDefinition::varying("bg_decay", 500.0),
        ],
        BackgroundKind::PowerLaw => vec![
            ParameterDefinition::varying("bg_amplitude", 0.0),
            ParameterDefinition::varying("bg_exponent", -1.0),
        ],
    }
}

fn find_definition<'a>(
    definitions: &'a [ParameterDefinition],
    expected_name: &str,
) -> Option<&'a ParameterDefinition> {
    let suffix = expected_name.strip_prefix("bg_").unwrap_or(expected_name);
    definitions
        .iter()
        .find(|definition| definition.name == expected_name || definition.name == suffix)
}

fn find_estimate<'a>(
    estimates: &'a [ParameterEstimate],
    expected_name: &str,
) -> Option<&'a ParameterEstimate> {
    estimates
        .iter()
        .find(|estimate| estimate.name == expected_name)
}

fn evaluate_background_estimates(
    kind: BackgroundKind,
    parameters: &[ParameterEstimate],
    x: &[f64],
) -> Result<Vec<f64>, FitError> {
    let value = |name: &str| {
        find_estimate(parameters, name)
            .map(|parameter| parameter.value)
            .ok_or_else(|| FitError::InvalidParameter {
                parameter: name.to_owned(),
            })
    };
    x.iter()
        .map(|independent| {
            let result = match kind {
                BackgroundKind::None => 0.0,
                BackgroundKind::Constant => value("bg_c")?,
                BackgroundKind::Linear => value("bg_slope")? * independent + value("bg_intercept")?,
                BackgroundKind::Quadratic => {
                    value("bg_a")? * independent * independent
                        + value("bg_b")? * independent
                        + value("bg_c")?
                }
                BackgroundKind::Exponential => {
                    value("bg_amplitude")? * (-independent / value("bg_decay")?).exp()
                }
                BackgroundKind::PowerLaw => {
                    value("bg_amplitude")? * independent.powf(value("bg_exponent")?)
                }
            };
            if result.is_finite() {
                Ok(result)
            } else {
                Err(FitError::NonFinite {
                    context: "background peak estimate".to_owned(),
                })
            }
        })
        .collect()
}

fn validate_data(x: &[f64], y: &[f64], bin_width: f64) -> Result<(), FitError> {
    if x.len() != y.len() {
        return Err(FitError::LengthMismatch {
            x: x.len(),
            y: y.len(),
        });
    }
    if x.is_empty() {
        return Err(FitError::EmptyData);
    }
    if !bin_width.is_finite() || bin_width <= 0.0 {
        return Err(FitError::Domain {
            model: "spectrum".to_owned(),
            message: "bin width must be positive and finite".to_owned(),
        });
    }
    if x.iter().chain(y).any(|value| !value.is_finite()) {
        return Err(FitError::NonFinite {
            context: "spectrum data".to_owned(),
        });
    }
    Ok(())
}

fn sorted_region(mut region: [f64; 2]) -> Result<[f64; 2], FitError> {
    if region.iter().any(|marker| !marker.is_finite()) || region[0] == region[1] {
        return Err(FitError::InvalidRegion);
    }
    if region[0] > region[1] {
        region.swap(0, 1);
    }
    Ok(region)
}

fn region_data(x: &[f64], y: &[f64], region: [f64; 2]) -> Result<(Vec<f64>, Vec<f64>), FitError> {
    let selected = x
        .iter()
        .copied()
        .zip(y.iter().copied())
        .filter(|(independent, _)| *independent >= region[0] && *independent <= region[1])
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(FitError::InvalidRegion);
    }
    Ok(selected.into_iter().unzip())
}

fn background_data(
    x: &[f64],
    y: &[f64],
    region: [f64; 2],
    bin_width: f64,
    markers: &[(f64, f64)],
    minimum_observations: usize,
) -> Result<(Vec<f64>, Vec<f64>), FitError> {
    let fallback = [
        (region[0] - bin_width, region[0]),
        (region[1], region[1] + bin_width),
    ];
    let windows = if markers.is_empty() {
        &fallback[..]
    } else {
        markers
    };
    let mut selected_x = Vec::new();
    let mut selected_y = Vec::new();
    let mut selected_indices = vec![false; x.len()];
    for &(first, second) in windows {
        if !first.is_finite() || !second.is_finite() {
            return Err(FitError::NonFinite {
                context: "background markers".to_owned(),
            });
        }
        let (lower, upper) = if first <= second {
            (first, second)
        } else {
            (second, first)
        };
        for (index, (independent, dependent)) in x.iter().zip(y).enumerate() {
            if *independent >= lower && *independent <= upper {
                selected_x.push(*independent);
                selected_y.push(*dependent);
                selected_indices[index] = true;
            }
        }
    }

    // Cursor-positioned region markers do not normally coincide with bin centers.
    // Consequently the one-bin-wide lmfit fallback can yield only two observations,
    // which is not enough to estimate a linear (or larger) model with uncertainty.
    // Preserve the exact fallback whenever it is sufficient; otherwise add the
    // closest unused bins to either region edge in a deterministic order.
    if markers.is_empty() && selected_x.len() < minimum_observations {
        let mut candidates = x
            .iter()
            .enumerate()
            .filter(|(index, _)| !selected_indices[*index])
            .map(|(index, independent)| {
                let distance = (*independent - region[0])
                    .abs()
                    .min((*independent - region[1]).abs());
                (index, distance)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(
            |(left_index, left_distance), (right_index, right_distance)| {
                left_distance
                    .total_cmp(right_distance)
                    .then_with(|| left_index.cmp(right_index))
            },
        );
        for (index, _) in candidates {
            selected_x.push(x[index]);
            selected_y.push(y[index]);
            if selected_x.len() >= minimum_observations {
                break;
            }
        }
    }
    if selected_x.is_empty() {
        return Err(FitError::InvalidRegion);
    }
    Ok((selected_x, selected_y))
}

fn estimate_sigma(x: &[f64], y: &[f64], marker: f64, bin_width: f64) -> f64 {
    let peak_index = nearest_index(x, marker);
    let half_maximum = y[peak_index] / 2.0;
    let left = y[..peak_index]
        .iter()
        .rposition(|value| *value <= half_maximum);
    let right = y[peak_index..]
        .iter()
        .position(|value| *value <= half_maximum)
        .map(|offset| peak_index + offset);
    match (left, right) {
        (Some(left), Some(right)) => ((x[right] - x[left]) / 2.3548).max(bin_width * 2.0),
        _ => bin_width * 2.0,
    }
}

fn nearest_index(values: &[f64], target: f64) -> usize {
    values
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            (*left - target)
                .abs()
                .partial_cmp(&(*right - target).abs())
                .unwrap_or(Ordering::Equal)
        })
        .map_or(0, |(index, _)| index)
}

fn center_bounds(index: usize, markers: &[f64], region: [f64; 2], sigma: f64) -> Bounds {
    if markers.len() == 1 {
        return Bounds::finite(region[0], region[1]);
    }
    let marker = markers[index];
    let previous = if index == 0 {
        region[0]
    } else {
        markers[index - 1]
    };
    let next = if index + 1 == markers.len() {
        region[1]
    } else {
        markers[index + 1]
    };
    let minimum = if (marker - previous).abs() <= sigma {
        previous
    } else {
        marker - sigma
    }
    .max(region[0]);
    let maximum = if (next - marker).abs() <= sigma {
        next
    } else {
        marker + sigma
    }
    .min(region[1]);
    Bounds::finite(minimum, maximum)
}

type SigmaLimits = (Vec<f64>, Vec<f64>);

fn normalized_sigma_bounds(
    bounds: Option<&SigmaBounds>,
    equal_sigma: bool,
    peaks: usize,
) -> Result<Option<SigmaLimits>, FitError> {
    let Some(bounds) = bounds else {
        return Ok(None);
    };
    let expected = if equal_sigma { 1 } else { peaks };
    if bounds.minima.len() != expected
        || bounds.maxima.len() != expected
        || bounds
            .minima
            .iter()
            .chain(&bounds.maxima)
            .any(|value| !value.is_finite())
        || bounds
            .minima
            .iter()
            .zip(&bounds.maxima)
            .any(|(minimum, maximum)| minimum < &0.0 || minimum >= maximum)
    {
        return Err(FitError::InvalidBounds {
            parameter: "sigma".to_owned(),
        });
    }
    Ok(Some((bounds.minima.clone(), bounds.maxima.clone())))
}

fn clamp_to_bounds(value: f64, bounds: Bounds) -> f64 {
    value
        .max(match bounds.lower {
            Bound::Unbounded => f64::NEG_INFINITY,
            Bound::Inclusive(lower) => lower,
        })
        .min(match bounds.upper {
            Bound::Unbounded => f64::INFINITY,
            Bound::Inclusive(upper) => upper,
        })
}

fn validate_power_law_domain(kind: BackgroundKind, x: &[f64]) -> Result<(), FitError> {
    if kind == BackgroundKind::PowerLaw && x.iter().any(|value| *value <= 0.0) {
        return Err(FitError::Domain {
            model: "power-law".to_owned(),
            message: "x must be positive for a real-valued variable exponent".to_owned(),
        });
    }
    Ok(())
}

fn linspace(start: f64, end: f64, count: usize) -> Result<Vec<f64>, FitError> {
    if count > 16_777_216 {
        return Err(FitError::AllocationLimit {
            requested: count,
            limit: 16_777_216,
        });
    }
    if count <= 1 {
        return Ok(vec![start]);
    }
    let denominator = (count - 1) as f64;
    Ok((0..count)
        .map(|index| start + (end - start) * index as f64 / denominator)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{
        BackgroundCoupling, BackgroundFitRequest, BackgroundKind, PeakFitRequest, fit_background,
        fit_peaks,
    };
    use crate::FitOptions;

    #[test]
    fn marker_free_background_expands_edge_sample_for_model_degrees_of_freedom() {
        let x = (0..=10).map(|index| index as f64 + 0.5).collect::<Vec<_>>();
        let linear_y = x
            .iter()
            .map(|independent| 0.75 * independent + 3.0)
            .collect::<Vec<_>>();
        let linear = fit_background(
            &BackgroundFitRequest {
                x: x.clone(),
                y: linear_y,
                bin_width: 1.0,
                region: [2.2, 8.8],
                markers: Vec::new(),
                kind: BackgroundKind::Linear,
                seed: None,
            },
            &FitOptions::default(),
        )
        .expect("automatic linear background sample should have positive degrees of freedom");
        assert_eq!(linear.statistics.observations, 3);
        let slope = linear
            .parameters
            .iter()
            .find(|parameter| parameter.name == "bg_slope")
            .expect("linear slope");
        assert!((slope.value - 0.75).abs() < 1.0e-10);

        let quadratic_y = x
            .iter()
            .map(|independent| 0.1 * independent * independent - 0.5 * independent + 7.0)
            .collect::<Vec<_>>();
        let quadratic = fit_background(
            &BackgroundFitRequest {
                x,
                y: quadratic_y,
                bin_width: 1.0,
                region: [2.2, 8.8],
                markers: Vec::new(),
                kind: BackgroundKind::Quadratic,
                seed: None,
            },
            &FitOptions::default(),
        )
        .expect("automatic quadratic background sample should have positive degrees of freedom");
        assert_eq!(quadratic.statistics.observations, 4);
        let coefficient = quadratic
            .parameters
            .iter()
            .find(|parameter| parameter.name == "bg_a")
            .expect("quadratic coefficient");
        assert!((coefficient.value - 0.1).abs() < 1.0e-9);
    }

    #[test]
    fn no_peak_or_background_markers_fit_peak_with_selected_background() {
        let x = (0..=100)
            .map(|index| index as f64 * 0.1 + 0.05)
            .collect::<Vec<_>>();
        let y = x
            .iter()
            .map(|independent| {
                2.0 + 0.2 * independent
                    + 80.0 / (2.506_628_274_631_000_2 * 0.35)
                        * (-0.5 * ((*independent - 5.0) / 0.35).powi(2)).exp()
            })
            .collect::<Vec<_>>();
        let result = fit_peaks(
            &PeakFitRequest {
                x,
                y,
                bin_width: 0.1,
                region: [2.03, 7.97],
                peak_markers: Vec::new(),
                background_markers: Vec::new(),
                background: BackgroundKind::Linear,
                background_seed: None,
                background_coupling: BackgroundCoupling::PrefitFrozen,
                equal_sigma: true,
                free_centers: true,
                sigma_bounds: None,
            },
            &FitOptions::default(),
        )
        .expect("marker-free peak and linear background fit should succeed");
        assert!(result.fit.termination.success);
        assert_eq!(result.peak_markers.len(), 1);
        assert!(
            result
                .fit
                .components
                .iter()
                .any(|component| component.name == "background")
        );
    }

    #[test]
    fn no_marker_uses_strongest_bin_and_fits_gaussian() {
        let x = (0..101).map(|index| index as f64 * 0.1).collect::<Vec<_>>();
        let y = x
            .iter()
            .map(|value| {
                3.0 + 100.0 / (2.506_628_274_631_000_2 * 0.4)
                    * (-0.5 * ((*value - 5.0) / 0.4).powi(2)).exp()
            })
            .collect::<Vec<_>>();
        let result = fit_peaks(
            &PeakFitRequest {
                x,
                y,
                bin_width: 0.1,
                region: [2.0, 8.0],
                peak_markers: Vec::new(),
                background_markers: vec![(2.0, 3.0), (7.0, 8.0)],
                background: BackgroundKind::Constant,
                background_seed: None,
                background_coupling: BackgroundCoupling::PrefitFrozen,
                equal_sigma: true,
                free_centers: true,
                sigma_bounds: None,
            },
            &FitOptions::default(),
        )
        .expect("synthetic fit should succeed");
        let center = result
            .fit
            .parameters
            .iter()
            .find(|parameter| parameter.name == "g0_center")
            .expect("center exists");
        assert!((center.value - 5.0).abs() < 1.0e-6);
        assert!(
            result.fit.covariance.is_some(),
            "termination={:?}, parameters={:?}",
            result.fit.termination,
            result.fit.parameters
        );
        assert!(result.fit.confidence_band.is_some());
    }
}
