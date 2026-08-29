use crate::egui_plot_stuff::{egui_line::EguiLine, egui_vertical_line::EguiVerticalLine};
use crate::fitter::common::Calibration;
use egui::{Color32, Id};
use egui_plot::{FilledArea, Line, LineStyle, PlotPoint, PlotPoints, PlotUi, VLine};
use serde::Deserialize;
use spectrix_fitting::{ManualPeakBounds, ManualPeakSeed, evaluate_manual_peak};
use std::cmp::Ordering;

use super::histogram1d::Histogram;
use crate::defaults::{MarkerDefaults, MarkerStyleDefaults};

fn apply_marker_style(line: &mut EguiVerticalLine, defaults: &MarkerStyleDefaults) {
    line.set_color(defaults.color);
    line.width = defaults.width;
    line.style = defaults.style;
    line.style_length = defaults.style_length;
    line.mid_point_radius = defaults.midpoint_radius;
}

const FWHM_FACTOR: f64 = 2.354_82;
/// Extra screen-space tolerance around draggable preview curves.
const PREVIEW_HIT_RADIUS_PX: f32 = 18.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PreviewHit {
    Seed,
    UpperBounds,
    LowerBounds,
    FwhmMinLower,
    FwhmMinUpper,
    FwhmMaxLower,
    FwhmMaxUpper,
    Position,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GuessSource {
    #[default]
    Estimated,
    Manual,
    Fitted,
}

impl GuessSource {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Estimated => "Estimated",
            Self::Manual => "Manual",
            Self::Fitted => "Fitted",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PeakGuess {
    pub id: u64,
    pub center: EguiVerticalLine,
    pub fwhm: f64,
    pub amplitude: f64,
    pub net_height: f64,
    pub center_min: f64,
    pub center_max: f64,
    pub fwhm_min: f64,
    pub fwhm_max: f64,
    pub net_height_min: f64,
    pub net_height_max: f64,
    pub width_source: GuessSource,
    pub amplitude_source: GuessSource,
    pub bounds_source: GuessSource,
    pub valid: bool,
    pub clean_width: bool,

    #[serde(skip)]
    pub left_dragging: bool,
    #[serde(skip)]
    pub right_dragging: bool,
    #[serde(skip)]
    pub apex_dragging: bool,
    #[serde(skip)]
    pub fwhm_min_dragging: bool,
    #[serde(skip)]
    pub fwhm_max_dragging: bool,
    #[serde(skip)]
    pub net_height_min_dragging: bool,
    #[serde(skip)]
    pub net_height_max_dragging: bool,
    #[serde(skip)]
    pub background_at_center: f64,
    #[serde(skip)]
    pub preview_hovered: bool,
    #[serde(skip)]
    pub preview_upper_bounds_hovered: bool,
    #[serde(skip)]
    pub preview_lower_bounds_hovered: bool,
    #[serde(skip)]
    pub seed_drag_start: Option<egui::Pos2>,
    #[serde(skip)]
    pub seed_drag_vertical: Option<bool>,
    #[serde(skip)]
    pub width_drag_upper: Option<bool>,
    #[serde(skip)]
    pub preview_position_hovered: bool,
    /// Legacy plain peak markers stored their position directly as `x_value`.
    /// Read it during migration but never write it into new saves.
    #[serde(default, skip_serializing, alias = "x_value")]
    legacy_x_value: Option<f64>,
}

impl Default for PeakGuess {
    fn default() -> Self {
        Self {
            id: 0,
            center: EguiVerticalLine::default(),
            fwhm: 0.0,
            amplitude: 0.0,
            net_height: 0.0,
            center_min: 0.0,
            center_max: 0.0,
            fwhm_min: 0.0,
            fwhm_max: 0.0,
            net_height_min: 0.0,
            net_height_max: 0.0,
            width_source: GuessSource::Estimated,
            amplitude_source: GuessSource::Estimated,
            bounds_source: GuessSource::Estimated,
            valid: false,
            clean_width: false,
            left_dragging: false,
            right_dragging: false,
            apex_dragging: false,
            fwhm_min_dragging: false,
            fwhm_max_dragging: false,
            net_height_min_dragging: false,
            net_height_max_dragging: false,
            background_at_center: 0.0,
            preview_hovered: false,
            preview_upper_bounds_hovered: false,
            preview_lower_bounds_hovered: false,
            seed_drag_start: None,
            seed_drag_vertical: None,
            width_drag_upper: None,
            preview_position_hovered: false,
            legacy_x_value: None,
        }
    }
}

impl PeakGuess {
    pub fn seed(&self) -> ManualPeakSeed {
        ManualPeakSeed {
            center: self.center.x_value,
            sigma: self.fwhm / FWHM_FACTOR,
            amplitude: self.amplitude,
        }
    }

    pub fn model_height(&self, bin_width: f64) -> f64 {
        evaluate_manual_peak(self.seed(), self.center.x_value, bin_width).unwrap_or(0.0)
    }

    pub(crate) fn set_net_height(&mut self, height: f64, bin_width: f64) -> bool {
        let unit_seed = ManualPeakSeed {
            center: self.center.x_value,
            sigma: self.fwhm / FWHM_FACTOR,
            amplitude: 1.0,
        };
        let Ok(response) = evaluate_manual_peak(unit_seed, self.center.x_value, bin_width) else {
            return false;
        };
        if response <= 0.0 || !response.is_finite() {
            return false;
        }
        self.net_height = height;
        self.amplitude = height / response;
        self.amplitude.is_finite() && self.amplitude > 0.0
    }

    pub(crate) fn set_fwhm_preserving_height(&mut self, fwhm: f64, bin_width: f64) -> bool {
        let held_height = self.net_height;
        self.fwhm = fwhm;
        self.set_net_height(held_height, bin_width)
    }

    pub fn bounds(&self) -> ManualPeakBounds {
        ManualPeakBounds {
            center: [self.center_min, self.center_max],
            sigma: [self.fwhm_min / FWHM_FACTOR, self.fwhm_max / FWHM_FACTOR],
            net_height: [self.net_height_min, self.net_height_max],
        }
    }

    pub(crate) fn bounds_valid(&self) -> bool {
        self.center_min.is_finite()
            && self.center_max.is_finite()
            && self.center_min < self.center_max
            && self.center.x_value >= self.center_min
            && self.center.x_value <= self.center_max
            && self.fwhm_min.is_finite()
            && self.fwhm_max.is_finite()
            && self.fwhm_min > 0.0
            && self.fwhm_min < self.fwhm_max
            && self.fwhm >= self.fwhm_min
            && self.fwhm <= self.fwhm_max
            && self.net_height_min.is_finite()
            && self.net_height_max.is_finite()
            && self.net_height_min >= 0.0
            && self.net_height_min < self.net_height_max
            && self.net_height >= self.net_height_min
            && self.net_height <= self.net_height_max
    }

    pub fn is_dragging(&self) -> bool {
        self.center.is_dragging
            || self.left_dragging
            || self.right_dragging
            || self.apex_dragging
            || self.fwhm_min_dragging
            || self.fwhm_max_dragging
            || self.net_height_min_dragging
            || self.net_height_max_dragging
    }

    pub fn reset_estimates(&mut self) {
        self.width_source = GuessSource::Estimated;
        self.amplitude_source = GuessSource::Estimated;
        self.bounds_source = GuessSource::Estimated;
        self.valid = false;
    }

    pub(crate) fn hold_current_seed(&mut self) {
        self.width_source = GuessSource::Manual;
        self.amplitude_source = GuessSource::Manual;
        self.bounds_source = GuessSource::Manual;
    }

    fn hold_bounds(&mut self) {
        self.bounds_source = GuessSource::Manual;
    }
}

fn deserialize_peak_guesses<'de, D>(deserializer: D) -> Result<Vec<PeakGuess>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut guesses = Vec::<PeakGuess>::deserialize(deserializer)?;
    for guess in &mut guesses {
        if let Some(x_value) = guess.legacy_x_value.take() {
            guess.center.x_value = x_value;
        }
    }
    Ok(guesses)
}

impl Histogram {
    pub fn update_background_pair_lines(&mut self) {
        // Extract bin edges and counts **before** modifying anything
        let bin_edges = self.get_bin_edges();
        let bin_counts = self.bins.clone();

        // Extract immutable background marker positions first
        let marker_positions: Vec<(f64, f64)> = self
            .plot_settings
            .markers
            .background_markers
            .iter()
            .map(|bg_pair| (bg_pair.start.x_value, bg_pair.end.x_value))
            .collect();

        // Compute bin indices based on marker positions **before** modifying anything
        let bin_indices: Vec<(usize, usize)> = marker_positions
            .iter()
            .map(|&(start_x, end_x)| {
                let start_bin = self.get_bin_index(start_x).unwrap_or(0);
                let end_bin = self
                    .get_bin_index(end_x)
                    .unwrap_or(self.bins.len().saturating_sub(1));
                (start_bin, end_bin)
            })
            .collect();

        // Now, modify `background_markers` without conflicting borrows
        for (bg_pair, &(start_bin, end_bin)) in self
            .plot_settings
            .markers
            .background_markers
            .iter_mut()
            .zip(bin_indices.iter())
        {
            bg_pair.histogram_line.clear_points(); // Clear previous points

            // Collect the **actual bin edges** and counts in the correct range
            for i in start_bin..=end_bin {
                if i < bin_edges.len() - 1 {
                    // Ensure no out-of-bounds access
                    let x_start = bin_edges[i]; // Start of the bin
                    let x_end = bin_edges[i + 1]; // End of the bin
                    let y = bin_counts[i] as f64; // Bin count

                    // Add both edges of the bin to the histogram line
                    bg_pair.histogram_line.add_point(x_start, y);
                    bg_pair.histogram_line.add_point(x_end, y);
                }
            }
        }
    }
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FitMarkers {
    pub region_markers: Vec<EguiVerticalLine>,
    #[serde(default, deserialize_with = "deserialize_peak_guesses")]
    pub peak_markers: Vec<PeakGuess>,
    pub background_markers: Vec<BackgroundPair>,
    #[serde(default)]
    pub defaults: MarkerDefaults,

    #[serde(skip)]
    pub cursor_position: Option<PlotPoint>,

    #[serde(skip)]
    pub manual_marker_position: f64,

    #[serde(default = "default_next_peak_id")]
    pub next_peak_id: u64,

    #[serde(skip)]
    pub estimate_signature: u64,

    /// Signature for an invalid-estimate retry already attempted. This prevents a
    /// failed estimator from retrying every frame until its inputs change.
    #[serde(skip)]
    pub invalid_estimate_signature: u64,

    #[serde(skip)]
    pub preview_background: Vec<[f64; 2]>,

    #[serde(skip)]
    pub last_equal_sigma: Option<bool>,

    #[serde(skip)]
    pub estimate_error: Option<String>,
}

const fn default_next_peak_id() -> u64 {
    1
}

#[derive(Default, Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackgroundPair {
    pub start: EguiVerticalLine,
    pub end: EguiVerticalLine,
    pub histogram_line: EguiLine,
}

impl BackgroundPair {
    pub fn is_dragging(&self) -> bool {
        self.start.is_dragging || self.end.is_dragging
    }

    pub fn new(start: EguiVerticalLine, end: EguiVerticalLine) -> Self {
        let mut line = EguiLine::new(egui::Color32::from_rgb(0, 200, 0));
        line.name = "Background Pair".to_owned();
        line.reference_fill = true;
        line.fill = 0.0;
        line.width = 0.0;
        line.fill_alpha = 0.05;

        line.add_point(start.x_value, 0.0);
        line.add_point(end.x_value, 0.0);

        Self {
            start,
            end,
            histogram_line: line,
        }
    }

    fn apply_defaults(&mut self, defaults: &MarkerDefaults) {
        apply_marker_style(&mut self.start, &defaults.background);
        apply_marker_style(&mut self.end, &defaults.background);
        self.histogram_line.set_color(defaults.background.color);
        self.histogram_line.fill_alpha = defaults.background_fill_alpha;
    }

    pub fn average_x(&self) -> f64 {
        (self.start.x_value + self.end.x_value) / 2.0
    }

    pub fn draw(&mut self, plot_ui: &mut PlotUi<'_>, calibration: Option<&Calibration>) {
        self.start.draw(plot_ui, calibration);
        self.end.draw(plot_ui, calibration);
        self.histogram_line.draw(plot_ui, calibration);
    }

    pub fn interactive_dragging(
        &mut self,
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
        raw_axis_range: (f64, f64),
    ) {
        self.start
            .interactive_dragging(plot_response, calibration, Some(raw_axis_range));
        self.end
            .interactive_dragging(plot_response, calibration, Some(raw_axis_range));
    }

    /// Updates the `histogram_line` to match the histogram bins within this background pair
    pub fn update_histogram_line(&mut self, bin_edges: &[f64], bin_counts: &[u32]) {
        let start_x = self.start.x_value;
        let end_x = self.end.x_value;

        let mut line_points = Vec::new();

        for (i, &edge) in bin_edges.iter().enumerate() {
            if edge >= start_x && edge <= end_x {
                let y_value = if i < bin_counts.len() {
                    bin_counts[i] as f64
                } else {
                    0.0
                };
                line_points.push([edge, y_value]);
            }
        }

        // Ensure the last point is included at the end marker
        if let Some(last_edge) = bin_edges.last()
            && *last_edge <= end_x
        {
            let last_count = *bin_counts.last().unwrap_or(&0) as f64;
            line_points.push([*last_edge, last_count]);
        }

        self.histogram_line.set_points(line_points);
    }
}

fn display_x_value(raw_x: f64, calibration: Option<&Calibration>, log_x: bool) -> Option<f64> {
    let calibrated = calibration.map_or(Some(raw_x), |cal| cal.calibrate_checked(raw_x))?;
    let value = if log_x && calibrated > 0.0 {
        calibrated.log10().max(0.0001)
    } else {
        calibrated
    };
    value.is_finite().then_some(value)
}

fn display_y_value(raw_y: f64, log_y: bool) -> Option<f64> {
    let value = if log_y && raw_y > 0.0 {
        raw_y.log10().max(0.0001)
    } else {
        raw_y
    };
    value.is_finite().then_some(value)
}

fn raw_x_value(
    display_x: f64,
    calibration: Option<&Calibration>,
    log_x: bool,
    raw_domain: (f64, f64),
    hint: f64,
) -> Option<f64> {
    let calibrated = if log_x {
        10.0f64.powf(display_x)
    } else {
        display_x
    };
    if let Some(calibration) = calibration {
        calibration.invert_in_range_with_hint(calibrated, raw_domain, Some(hint))
    } else {
        calibrated.is_finite().then_some(calibrated)
    }
}

fn raw_y_value(display_y: f64, log_y: bool) -> Option<f64> {
    let raw = if log_y {
        10.0f64.powf(display_y)
    } else {
        display_y
    };
    raw.is_finite().then_some(raw)
}

fn interpolate_background(points: &[[f64; 2]], x: f64) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let upper = points.partition_point(|point| point[0] < x);
    match upper {
        0 => points[0][1],
        upper if upper >= points.len() => points[points.len() - 1][1],
        upper => {
            let [x0, y0] = points[upper - 1];
            let [x1, y1] = points[upper];
            if (x1 - x0).abs() <= f64::EPSILON {
                y0
            } else {
                y0 + (x - x0) / (x1 - x0) * (y1 - y0)
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct PeakInteraction {
    changed: bool,
    center_moved: bool,
    shared_fwhm: Option<f64>,
    shared_fwhm_bounds: Option<[f64; 2]>,
}

impl PeakGuess {
    fn ids(&self) -> (Id, Id, Id, Id, Id, Id, Id, Id) {
        (
            Id::new(("manual-peak-center", self.id)),
            Id::new(("manual-peak-fwhm-left", self.id)),
            Id::new(("manual-peak-fwhm-right", self.id)),
            Id::new(("manual-peak-apex", self.id)),
            Id::new(("manual-peak-fwhm-min", self.id)),
            Id::new(("manual-peak-fwhm-max", self.id)),
            Id::new(("manual-peak-height-min", self.id)),
            Id::new(("manual-peak-height-max", self.id)),
        )
    }

    fn draw(
        &mut self,
        plot_ui: &mut PlotUi<'_>,
        calibration: Option<&Calibration>,
        log_x: bool,
        log_y: bool,
        bin_width: f64,
        background: &[[f64; 2]],
    ) {
        let seed = self.seed();
        if !seed.center.is_finite() {
            return;
        }
        self.background_at_center = interpolate_background(background, seed.center);
        let Some(center_x) = display_x_value(seed.center, calibration, log_x) else {
            return;
        };
        let (center_id, ..) = self.ids();
        let color = self.center.color;
        let preview_color = Color32::from_rgb(245, 145, 30);

        if self.valid && self.fwhm.is_finite() && self.fwhm > 0.0 {
            let raw_left = seed.center - 0.5 * self.fwhm;
            let raw_right = seed.center + 0.5 * self.fwhm;
            if display_x_value(raw_left, calibration, log_x).is_some()
                && display_x_value(raw_right, calibration, log_x).is_some()
            {
                let seed_for = |fwhm: f64, height: f64| {
                    let sigma = fwhm / FWHM_FACTOR;
                    let unit = evaluate_manual_peak(
                        ManualPeakSeed {
                            center: seed.center,
                            sigma,
                            amplitude: 1.0,
                        },
                        seed.center,
                        bin_width,
                    )
                    .ok()?;
                    (unit > 0.0).then_some(ManualPeakSeed {
                        center: seed.center,
                        sigma,
                        amplitude: height / unit,
                    })
                };
                let corner_seeds = [
                    seed_for(self.fwhm_min, self.net_height_min),
                    seed_for(self.fwhm_min, self.net_height_max),
                    seed_for(self.fwhm_max, self.net_height_min),
                    seed_for(self.fwhm_max, self.net_height_max),
                ];
                if self.bounds_valid() && corner_seeds.iter().all(Option::is_some) {
                    let widest_sigma = self.fwhm_max / FWHM_FACTOR;
                    let start = seed.center - 5.0 * widest_sigma;
                    let end = seed.center + 5.0 * widest_sigma;
                    let mut xs = Vec::with_capacity(141);
                    let mut lower = Vec::with_capacity(141);
                    let mut upper = Vec::with_capacity(141);
                    for index in 0..=140 {
                        let raw_x = start + (end - start) * index as f64 / 140.0;
                        let baseline = interpolate_background(background, raw_x);
                        let mut minimum = f64::INFINITY;
                        let mut maximum = f64::NEG_INFINITY;
                        for corner in corner_seeds.iter().flatten() {
                            if let Ok(value) = evaluate_manual_peak(*corner, raw_x, bin_width) {
                                minimum = minimum.min(baseline + value);
                                maximum = maximum.max(baseline + value);
                            }
                        }
                        let Some(display_x) = display_x_value(raw_x, calibration, log_x) else {
                            continue;
                        };
                        let (Some(display_minimum), Some(display_maximum)) = (
                            display_y_value(minimum, log_y),
                            display_y_value(maximum, log_y),
                        ) else {
                            continue;
                        };
                        xs.push(display_x);
                        lower.push(display_minimum);
                        upper.push(display_maximum);
                    }
                    if xs.len() >= 2 {
                        plot_ui.add(
                            FilledArea::new("", &xs, &lower, &upper)
                                .id(Id::new(("manual-peak-preview-envelope", self.id)))
                                .allow_hover(false)
                                .fill_color(preview_color.gamma_multiply(
                                    if self.preview_position_hovered || self.center.is_dragging {
                                        0.24
                                    } else {
                                        0.12
                                    },
                                )),
                        );
                        for (edge, highlighted) in [
                            (
                                &lower,
                                self.preview_lower_bounds_hovered
                                    || self.width_drag_upper == Some(false)
                                    || self.net_height_min_dragging,
                            ),
                            (
                                &upper,
                                self.preview_upper_bounds_hovered
                                    || self.width_drag_upper == Some(true)
                                    || self.net_height_max_dragging,
                            ),
                        ] {
                            plot_ui.line(
                                Line::new(
                                    "",
                                    PlotPoints::Owned(
                                        xs.iter()
                                            .zip(edge)
                                            .map(|(&x, &y)| PlotPoint::new(x, y))
                                            .collect(),
                                    ),
                                )
                                .allow_hover(false)
                                .color(preview_color.gamma_multiply(if highlighted {
                                    1.0
                                } else {
                                    0.45
                                }))
                                .width(if highlighted {
                                    2.2
                                } else {
                                    0.9
                                }),
                            );
                        }
                    }
                }

                let start = seed.center - 5.0 * (self.fwhm / FWHM_FACTOR);
                let end = seed.center + 5.0 * (self.fwhm / FWHM_FACTOR);
                let points = (0..=140)
                    .filter_map(|index| {
                        let raw_x = start + (end - start) * index as f64 / 140.0;
                        let y = interpolate_background(background, raw_x)
                            + evaluate_manual_peak(seed, raw_x, bin_width).ok()?;
                        Some([
                            display_x_value(raw_x, calibration, log_x)?,
                            display_y_value(y, log_y)?,
                        ])
                    })
                    .collect::<Vec<_>>();
                if points.len() >= 2 {
                    let seed_highlighted = self.preview_hovered
                        || self.apex_dragging
                        || self.left_dragging
                        || self.right_dragging;
                    plot_ui.line(
                        Line::new(
                            "",
                            PlotPoints::Owned(points.into_iter().map(Into::into).collect()),
                        )
                        .id(Id::new(("manual-peak-preview-seed", self.id)))
                        .allow_hover(false)
                        .highlight(seed_highlighted)
                        .color(preview_color.gamma_multiply(if seed_highlighted {
                            1.0
                        } else {
                            0.55
                        }))
                        .width(if seed_highlighted { 2.0 } else { 0.8 })
                        .style(LineStyle::Dashed { length: 4.0 }),
                    );
                }
            }
        }

        plot_ui.vline(
            VLine::new("", center_x)
                .id(center_id)
                .allow_hover(true)
                .highlight(self.center.is_dragging)
                .stroke(self.center.stroke)
                .width(self.center.width)
                .color(color)
                .style(self.center.style.to_egui(self.center.style_length)),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn preview_hit(
        &self,
        pointer_position: egui::Pos2,
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
        raw_axis_range: (f64, f64),
        log_x: bool,
        log_y: bool,
        bin_width: f64,
        background: &[[f64; 2]],
    ) -> Option<PreviewHit> {
        if !self.valid || self.fwhm <= 0.0 {
            return None;
        }
        let plot_point = plot_response
            .transform
            .value_from_position(pointer_position);
        let Some(raw_x) = raw_x_value(
            plot_point.x,
            calibration,
            log_x,
            raw_axis_range,
            self.center.x_value,
        ) else {
            return None;
        };

        let seed_for = |fwhm: f64, height: f64| {
            let sigma = fwhm / FWHM_FACTOR;
            let unit = evaluate_manual_peak(
                ManualPeakSeed {
                    center: self.center.x_value,
                    sigma,
                    amplitude: 1.0,
                },
                self.center.x_value,
                bin_width,
            )
            .ok()?;
            (unit > 0.0).then_some(ManualPeakSeed {
                center: self.center.x_value,
                sigma,
                amplitude: height / unit,
            })
        };
        let baseline = interpolate_background(background, raw_x);
        let Some(seed_y) = evaluate_manual_peak(self.seed(), raw_x, bin_width)
            .ok()
            .and_then(|height| display_y_value(baseline + height, log_y))
        else {
            return None;
        };
        let seed_position = plot_response
            .transform
            .position_from_point(&PlotPoint::new(plot_point.x, seed_y));
        if pointer_position.distance(seed_position) <= PREVIEW_HIT_RADIUS_PX {
            return Some(PreviewHit::Seed);
        }

        let mut envelope_ys = Vec::with_capacity(4);
        let offset = (raw_x - self.center.x_value).abs();
        let mut width_hit = None;
        for (seed, hit) in [
            (
                seed_for(self.fwhm_min, self.net_height_min),
                PreviewHit::FwhmMinLower,
            ),
            (
                seed_for(self.fwhm_min, self.net_height_max),
                PreviewHit::FwhmMinUpper,
            ),
            (
                seed_for(self.fwhm_max, self.net_height_min),
                PreviewHit::FwhmMaxLower,
            ),
            (
                seed_for(self.fwhm_max, self.net_height_max),
                PreviewHit::FwhmMaxUpper,
            ),
        ]
        .into_iter()
        .filter_map(|(seed, hit)| seed.map(|seed| (seed, hit)))
        {
            let Some(display_y) = evaluate_manual_peak(seed, raw_x, bin_width)
                .ok()
                .and_then(|height| display_y_value(baseline + height, log_y))
            else {
                continue;
            };
            let position = plot_response
                .transform
                .position_from_point(&PlotPoint::new(plot_point.x, display_y));
            if offset > 0.15 * self.fwhm
                && pointer_position.distance(position) <= PREVIEW_HIT_RADIUS_PX
            {
                width_hit = Some(hit);
            }
            envelope_ys.push(display_y);
        }
        if envelope_ys.len() != 4 {
            return None;
        }
        let half_span = 5.0 * self.fwhm_max / FWHM_FACTOR;
        let low = envelope_ys.iter().copied().fold(f64::INFINITY, f64::min);
        let high = envelope_ys
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        if (raw_x - self.center.x_value).abs() > half_span || !(low..=high).contains(&plot_point.y)
        {
            return None;
        }
        let lower_edge = plot_response
            .transform
            .position_from_point(&PlotPoint::new(plot_point.x, low));
        let upper_edge = plot_response
            .transform
            .position_from_point(&PlotPoint::new(plot_point.x, high));
        if let Some(hit) = width_hit {
            Some(hit)
        } else if pointer_position.distance(upper_edge) <= PREVIEW_HIT_RADIUS_PX {
            Some(PreviewHit::UpperBounds)
        } else if pointer_position.distance(lower_edge) <= PREVIEW_HIT_RADIUS_PX {
            Some(PreviewHit::LowerBounds)
        } else {
            Some(PreviewHit::Position)
        }
    }

    fn interactive_dragging(
        &mut self,
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
        raw_axis_range: (f64, f64),
        log_x: bool,
        log_y: bool,
        bin_width: f64,
        background: &[[f64; 2]],
        preview_owner: Option<u64>,
    ) -> PeakInteraction {
        let pointer = plot_response
            .response
            .ctx
            .input(|input| input.pointer.clone());
        let (
            center_id,
            left_id,
            right_id,
            apex_id,
            fwhm_min_id,
            fwhm_max_id,
            height_min_id,
            height_max_id,
        ) = self.ids();
        let mut interaction = PeakInteraction::default();
        if let Some(pointer_position) = pointer.hover_pos() {
            let mut grabbed_explicit_handle = false;
            if let Some(hovered) = plot_response.hovered_plot_item {
                grabbed_explicit_handle = hovered == center_id
                    || hovered == left_id
                    || hovered == right_id
                    || hovered == apex_id
                    || hovered == fwhm_min_id
                    || hovered == fwhm_max_id
                    || hovered == height_min_id
                    || hovered == height_max_id;
                if pointer.button_pressed(egui::PointerButton::Primary) {
                    self.center.is_dragging = hovered == center_id;
                    self.left_dragging = hovered == left_id;
                    self.right_dragging = hovered == right_id;
                    self.apex_dragging = hovered == apex_id;
                    self.fwhm_min_dragging = hovered == fwhm_min_id;
                    self.fwhm_max_dragging = hovered == fwhm_max_id;
                    self.net_height_min_dragging = hovered == height_min_id;
                    self.net_height_max_dragging = hovered == height_max_id;
                }
                self.center.highlighted = hovered == center_id;
            }

            let plot_point = plot_response
                .transform
                .value_from_position(pointer_position);
            let preview_hit = (!grabbed_explicit_handle
                && preview_owner.is_none_or(|id| id == self.id))
            .then(|| {
                self.preview_hit(
                    pointer_position,
                    plot_response,
                    calibration,
                    raw_axis_range,
                    log_x,
                    log_y,
                    bin_width,
                    background,
                )
            })
            .flatten();
            self.preview_hovered = preview_hit == Some(PreviewHit::Seed);
            self.preview_upper_bounds_hovered = matches!(
                preview_hit,
                Some(PreviewHit::UpperBounds | PreviewHit::FwhmMinUpper | PreviewHit::FwhmMaxUpper)
            );
            self.preview_lower_bounds_hovered = matches!(
                preview_hit,
                Some(PreviewHit::LowerBounds | PreviewHit::FwhmMinLower | PreviewHit::FwhmMaxLower)
            );
            self.preview_position_hovered = preview_hit == Some(PreviewHit::Position);
            if pointer.button_pressed(egui::PointerButton::Primary)
                && let Some(preview_hit) = preview_hit
            {
                match preview_hit {
                    PreviewHit::Seed => {
                        self.seed_drag_start = Some(pointer_position);
                        self.seed_drag_vertical = None;
                    }
                    PreviewHit::UpperBounds => self.net_height_max_dragging = true,
                    PreviewHit::LowerBounds => self.net_height_min_dragging = true,
                    PreviewHit::FwhmMinLower | PreviewHit::FwhmMinUpper => {
                        self.fwhm_min_dragging = true;
                        self.width_drag_upper = Some(preview_hit == PreviewHit::FwhmMinUpper);
                    }
                    PreviewHit::FwhmMaxLower | PreviewHit::FwhmMaxUpper => {
                        self.fwhm_max_dragging = true;
                        self.width_drag_upper = Some(preview_hit == PreviewHit::FwhmMaxUpper);
                    }
                    PreviewHit::Position => self.center.is_dragging = true,
                }
            }
            if let Some(start) = self.seed_drag_start
                && self.seed_drag_vertical.is_none()
            {
                let delta = pointer_position - start;
                if delta.length_sq() >= 9.0 {
                    let vertical = delta.y.abs() >= delta.x.abs();
                    self.seed_drag_vertical = Some(vertical);
                    self.apex_dragging = vertical;
                    self.right_dragging = !vertical;
                }
            }
            if self.center.is_dragging
                && let Some(raw_x) = raw_x_value(
                    plot_point.x,
                    calibration,
                    log_x,
                    raw_axis_range,
                    self.center.x_value,
                )
            {
                let (minimum, maximum) = if self.center_min < self.center_max {
                    (self.center_min, self.center_max)
                } else {
                    (
                        raw_axis_range.0.min(raw_axis_range.1),
                        raw_axis_range.0.max(raw_axis_range.1),
                    )
                };
                self.center.x_value = raw_x.clamp(minimum, maximum);
                self.hold_current_seed();
                interaction.changed = true;
                interaction.center_moved = true;
            }
            if (self.left_dragging || self.right_dragging)
                && let Some(raw_x) = raw_x_value(
                    plot_point.x,
                    calibration,
                    log_x,
                    raw_axis_range,
                    self.center.x_value,
                )
            {
                let minimum = if self.fwhm_min > 0.0 {
                    self.fwhm_min
                } else {
                    (0.5 * bin_width).max(f64::EPSILON)
                };
                let maximum = if self.fwhm_max > minimum {
                    self.fwhm_max
                } else {
                    (raw_axis_range.1 - raw_axis_range.0).abs().max(minimum)
                };
                let fwhm = (2.0 * (raw_x - self.center.x_value).abs()).clamp(minimum, maximum);
                self.set_fwhm_preserving_height(fwhm, bin_width);
                self.hold_current_seed();
                self.valid =
                    self.amplitude.is_finite() && self.amplitude > 0.0 && self.bounds_valid();
                interaction.shared_fwhm = Some(fwhm);
                interaction.changed = true;
            }
            if (self.fwhm_min_dragging || self.fwhm_max_dragging)
                && let Some(raw_x) = raw_x_value(
                    plot_point.x,
                    calibration,
                    log_x,
                    raw_axis_range,
                    self.center.x_value,
                )
            {
                let value = 2.0 * (raw_x - self.center.x_value).abs();
                let absolute_minimum = (0.5 * bin_width).max(f64::EPSILON);
                if self.fwhm_min_dragging {
                    self.fwhm_min = value.clamp(absolute_minimum, self.fwhm);
                } else {
                    let absolute_maximum =
                        (raw_axis_range.1 - raw_axis_range.0).abs().max(self.fwhm);
                    self.fwhm_max = value.clamp(self.fwhm, absolute_maximum);
                }
                self.hold_bounds();
                self.valid =
                    self.amplitude.is_finite() && self.amplitude > 0.0 && self.bounds_valid();
                interaction.shared_fwhm_bounds = Some([self.fwhm_min, self.fwhm_max]);
                interaction.changed = true;
            }
            if self.apex_dragging
                && let Some(raw_y) = raw_y_value(plot_point.y, log_y)
            {
                let target_height = (raw_y - self.background_at_center).max(0.0).clamp(
                    self.net_height_min.max(0.0),
                    self.net_height_max.max(self.net_height_min),
                );
                if self.set_net_height(target_height, bin_width) {
                    self.hold_current_seed();
                    self.valid =
                        self.amplitude.is_finite() && self.amplitude > 0.0 && self.bounds_valid();
                    interaction.changed = true;
                }
            }
            if (self.net_height_min_dragging || self.net_height_max_dragging)
                && let Some(raw_y) = raw_y_value(plot_point.y, log_y)
            {
                let value = (raw_y - self.background_at_center).max(0.0);
                if self.net_height_min_dragging {
                    self.net_height_min = value.min(self.net_height);
                } else {
                    self.net_height_max = value.max(self.net_height);
                }
                self.hold_bounds();
                self.valid =
                    self.amplitude.is_finite() && self.amplitude > 0.0 && self.bounds_valid();
                interaction.changed = true;
            }
        }

        // Plot responses may miss the one-frame `button_released` transition when a drag ends
        // outside their active area. Clear every mode whenever the primary button is up so a
        // later click can select the curve or band afresh.
        if !pointer.primary_down() {
            self.center.is_dragging = false;
            self.left_dragging = false;
            self.right_dragging = false;
            self.apex_dragging = false;
            self.fwhm_min_dragging = false;
            self.fwhm_max_dragging = false;
            self.net_height_min_dragging = false;
            self.net_height_max_dragging = false;
            self.seed_drag_start = None;
            self.seed_drag_vertical = None;
            self.width_drag_upper = None;
        }
        if pointer.hover_pos().is_none() {
            self.preview_hovered = false;
            self.preview_upper_bounds_hovered = false;
            self.preview_lower_bounds_hovered = false;
            self.preview_position_hovered = false;
        }
        interaction
    }
}

impl FitMarkers {
    pub fn new() -> Self {
        Self {
            next_peak_id: default_next_peak_id(),
            ..Self::default()
        }
    }

    pub fn ensure_peak_ids(&mut self) {
        let mut next = self
            .peak_markers
            .iter()
            .map(|guess| guess.id)
            .max()
            .unwrap_or(0)
            .saturating_add(1)
            .max(self.next_peak_id)
            .max(1);
        let mut seen = std::collections::HashSet::new();
        for guess in &mut self.peak_markers {
            if guess.id == 0 || !seen.insert(guess.id) {
                guess.id = next;
                seen.insert(next);
                next = next.saturating_add(1);
            }
            guess.center.name = format!("Peak Marker {}", guess.id);
        }
        self.next_peak_id = next;
    }

    /// Highest upper initial-guess envelope value whose peak center is visible.
    /// The envelope reaches its maximum at the peak center, where its FWHM does not affect
    /// the height, so this is sufficient for plot Y-range fitting.
    pub fn visible_peak_preview_max(&self, raw_x_min: f64, raw_x_max: f64) -> Option<f64> {
        let minimum = raw_x_min.min(raw_x_max);
        let maximum = raw_x_min.max(raw_x_max);
        self.peak_markers
            .iter()
            .filter(|peak| {
                peak.valid
                    && peak.net_height_max.is_finite()
                    && peak.center.x_value >= minimum
                    && peak.center.x_value <= maximum
            })
            .map(|peak| {
                interpolate_background(&self.preview_background, peak.center.x_value)
                    + peak.net_height_max
            })
            .filter(|height| height.is_finite())
            .max_by(|left, right| left.total_cmp(right))
    }

    pub fn is_dragging(&self) -> bool {
        self.region_markers.iter().any(|m| m.is_dragging)
            || self.peak_markers.iter().any(PeakGuess::is_dragging)
            || self.background_markers.iter().any(|m| m.is_dragging())
    }

    pub fn apply_defaults(&mut self, defaults: &MarkerDefaults) {
        self.defaults = defaults.clone();
        for marker in &mut self.region_markers {
            apply_marker_style(marker, &defaults.region);
        }
        for marker in &mut self.peak_markers {
            apply_marker_style(&mut marker.center, &defaults.peak);
        }
        for pair in &mut self.background_markers {
            pair.apply_defaults(defaults);
        }
    }

    pub fn add_region_marker(&mut self, x: f64) {
        if self.region_markers.len() >= 2 {
            self.clear_region_markers();
        }

        let mut marker = EguiVerticalLine::new(x, self.defaults.region.color);
        apply_marker_style(&mut marker, &self.defaults.region);
        marker.name = format!("Region Marker (x={x:.2})");

        self.region_markers.push(marker);

        self.region_markers.sort_by(
            |a, b| match (a.x_value.is_finite(), b.x_value.is_finite()) {
                (true, true) => a.x_value.total_cmp(&b.x_value),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => Ordering::Equal,
            },
        );
        self.reset_all_estimates();
    }

    pub fn add_peak_marker(&mut self, x: f64) {
        self.ensure_peak_ids();
        let mut marker = EguiVerticalLine::new(x, self.defaults.peak.color);
        apply_marker_style(&mut marker, &self.defaults.peak);
        let id = self.next_peak_id.max(1);
        self.next_peak_id = id.saturating_add(1);
        marker.name = format!("Peak Marker {id}");

        self.peak_markers.push(PeakGuess {
            id,
            center: marker,
            ..PeakGuess::default()
        });
        self.peak_markers.sort_by(|a, b| {
            match (a.center.x_value.is_finite(), b.center.x_value.is_finite()) {
                (true, true) => a.center.x_value.total_cmp(&b.center.x_value),
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                (false, false) => Ordering::Equal,
            }
        });
    }

    pub fn add_peak_seed(
        &mut self,
        seed: ManualPeakSeed,
        source: GuessSource,
        bin_width: f64,
        bounds: Option<ManualPeakBounds>,
    ) {
        self.add_peak_marker(seed.center);
        if let Some(guess) = self
            .peak_markers
            .iter_mut()
            .find(|guess| guess.center.x_value == seed.center)
        {
            guess.fwhm = FWHM_FACTOR * seed.sigma;
            guess.amplitude = seed.amplitude;
            guess.net_height = guess.model_height(bin_width);
            guess.center_min = seed.center - seed.sigma.max(0.5 * bin_width);
            guess.center_max = seed.center + seed.sigma.max(0.5 * bin_width);
            guess.fwhm_min = 0.67 * guess.fwhm;
            guess.fwhm_max = 1.5 * guess.fwhm;
            let height_delta = guess
                .net_height
                .max(1.0)
                .sqrt()
                .max(0.15 * guess.net_height);
            guess.net_height_min = (guess.net_height - height_delta).max(0.0);
            guess.net_height_max = guess.net_height + height_delta;
            if let Some(bounds) = bounds {
                guess.center_min = bounds.center[0];
                guess.center_max = bounds.center[1];
                guess.fwhm_min = FWHM_FACTOR * bounds.sigma[0];
                guess.fwhm_max = FWHM_FACTOR * bounds.sigma[1];
                guess.net_height_min = bounds.net_height[0];
                guess.net_height_max = bounds.net_height[1];
            }
            guess.width_source = source;
            guess.amplitude_source = source;
            guess.bounds_source = source;
            guess.valid = seed.center.is_finite()
                && seed.sigma.is_finite()
                && seed.sigma > 0.0
                && seed.amplitude.is_finite()
                && seed.amplitude > 0.0
                && guess.bounds_valid();
        }
        self.estimate_signature = 0;
    }

    pub fn add_background_pair(&mut self, x: f64, bin_width: f64) {
        let mut marker_start = EguiVerticalLine::new(x, self.defaults.background.color);
        let mut marker_end = EguiVerticalLine::new(x, self.defaults.background.color);
        apply_marker_style(&mut marker_start, &self.defaults.background);
        apply_marker_style(&mut marker_end, &self.defaults.background);

        marker_start.name = format!("Background Pair {} Start", self.background_markers.len());
        marker_end.name = format!("Background Pair {} End", self.background_markers.len());

        marker_start.x_value = x;
        marker_end.x_value = x + bin_width;

        let mut markers = BackgroundPair::new(marker_start, marker_end);
        markers.apply_defaults(&self.defaults);
        self.background_markers.push(markers);
    }

    pub fn clear_region_markers(&mut self) {
        self.region_markers.clear();
        self.reset_all_estimates();
    }

    pub fn clear_peak_markers(&mut self) {
        self.peak_markers.clear();
        self.estimate_signature = 0;
    }

    pub fn clear_background_markers(&mut self) {
        self.background_markers.clear();
    }

    pub fn set_background_marker_positions(&mut self, markers: &[(f64, f64)]) {
        self.background_markers.clear();

        for (idx, &(start, end)) in markers.iter().enumerate() {
            let mut marker_start = EguiVerticalLine::new(start, self.defaults.background.color);
            let mut marker_end = EguiVerticalLine::new(end, self.defaults.background.color);
            apply_marker_style(&mut marker_start, &self.defaults.background);
            apply_marker_style(&mut marker_end, &self.defaults.background);
            marker_start.name = format!("Background Pair {idx} Start");
            marker_end.name = format!("Background Pair {idx} End");

            let mut pair = BackgroundPair::new(marker_start, marker_end);
            pair.apply_defaults(&self.defaults);
            self.background_markers.push(pair);
        }
    }

    fn delete_marker(markers: &mut Vec<EguiVerticalLine>, marker_to_delete: f64) {
        if let Some(index) = markers.iter().position(|x| x.x_value == marker_to_delete) {
            markers.remove(index);
        }
    }

    fn delete_peak_marker(&mut self, marker_to_delete: f64) {
        if let Some(index) = self
            .peak_markers
            .iter()
            .position(|guess| guess.center.x_value == marker_to_delete)
        {
            self.peak_markers.remove(index);
            self.reset_all_estimates();
        }
    }

    /// Mark every remaining peak for a fresh data-driven estimate.
    ///
    /// Region bounds and the set of peaks are shared fit inputs, so changing either
    /// invalidates estimates for every peak, not only the marker that changed.
    pub fn reset_all_estimates(&mut self) {
        for guess in &mut self.peak_markers {
            guess.reset_estimates();
        }
        self.estimate_signature = 0;
        self.invalid_estimate_signature = 0;
    }

    pub fn delete_closest_marker(&mut self, cursor_x: f64) {
        let mut all_markers: Vec<(f64, &str)> = vec![];

        all_markers.extend(self.region_markers.iter().map(|x| (x.x_value, "region")));
        all_markers.extend(
            self.peak_markers
                .iter()
                .map(|guess| (guess.center.x_value, "peak")),
        );
        all_markers.extend(
            self.background_markers
                .iter()
                .map(|x| (x.average_x(), "background")),
        );

        if let Some(&(closest_marker, marker_type)) =
            all_markers.iter().min_by(|(x1, _), (x2, _)| {
                let dist1 = (cursor_x - x1).abs();
                let dist2 = (cursor_x - x2).abs();
                dist1.total_cmp(&dist2)
            })
        {
            match marker_type {
                "region" => {
                    Self::delete_marker(&mut self.region_markers, closest_marker);
                    self.reset_all_estimates();
                }
                "peak" => self.delete_peak_marker(closest_marker),
                "background" => {
                    self.background_markers
                        .retain(|x| x.average_x() != closest_marker);
                }
                _ => {}
            }
        }
    }

    fn get_marker_positions(markers: &[EguiVerticalLine]) -> Vec<f64> {
        markers.iter().map(|m| m.x_value).collect()
    }

    pub fn get_region_marker_positions(&self) -> Vec<f64> {
        Self::get_marker_positions(&self.region_markers)
    }

    pub fn get_peak_marker_positions(&self) -> Vec<f64> {
        self.peak_markers
            .iter()
            .map(|guess| guess.center.x_value)
            .collect()
    }

    pub fn get_peak_seeds(&self) -> Vec<ManualPeakSeed> {
        self.peak_markers.iter().map(PeakGuess::seed).collect()
    }

    pub fn get_peak_bounds(&self) -> Vec<ManualPeakBounds> {
        self.peak_markers.iter().map(PeakGuess::bounds).collect()
    }

    pub fn get_background_marker_positions(&self) -> Vec<(f64, f64)> {
        // Self::get_marker_positions(&self.background_markers)
        self.background_markers
            .iter()
            .map(|m| (m.start.x_value, m.end.x_value))
            .collect()
    }

    pub fn remove_peak_markers_outside_region(&mut self) {
        let before = self.peak_markers.len();
        self.peak_markers.retain(|peak| {
            self.region_markers
                .first()
                .is_some_and(|start| peak.center.x_value >= start.x_value)
                && self
                    .region_markers
                    .get(1)
                    .is_some_and(|end| peak.center.x_value <= end.x_value)
        });
        if self.peak_markers.len() != before {
            self.reset_all_estimates();
        }
    }

    pub fn draw_all_markers(
        &mut self,
        plot_ui: &mut PlotUi<'_>,
        calibration: Option<&Calibration>,
        log_x: bool,
        log_y: bool,
        bin_width: f64,
    ) {
        self.ensure_peak_ids();
        for marker in &mut self.background_markers {
            marker.draw(plot_ui, calibration);
        }

        for marker in &mut self.region_markers {
            marker.draw(plot_ui, calibration);
        }

        for marker in &mut self.peak_markers {
            marker.draw(
                plot_ui,
                calibration,
                log_x,
                log_y,
                bin_width,
                &self.preview_background,
            );
        }
    }

    pub fn interactive_dragging(
        &mut self,
        plot_response: &egui_plot::PlotResponse<()>,
        calibration: Option<&Calibration>,
        raw_axis_range: (f64, f64),
        log_x: bool,
        log_y: bool,
        bin_width: f64,
        equal_sigma: bool,
        auto_estimate_moved_peak: bool,
    ) -> bool {
        self.ensure_peak_ids();
        let before_background = self.get_background_marker_positions();
        let before_regions = self.get_region_marker_positions();
        for marker in &mut self.background_markers {
            marker.interactive_dragging(plot_response, calibration, raw_axis_range);
        }

        for marker in &mut self.region_markers {
            marker.interactive_dragging(plot_response, calibration, Some(raw_axis_range));
        }

        let mut changed = false;
        let mut moved_peak_ids = Vec::new();
        let mut shared_fwhm = None;
        let mut shared_fwhm_bounds = None;
        let pointer = plot_response
            .response
            .ctx
            .input(|input| input.pointer.clone());
        let preview_owner = pointer
            .button_pressed(egui::PointerButton::Primary)
            .then(|| pointer.hover_pos())
            .flatten()
            .and_then(|pointer_position| {
                self.peak_markers
                    .iter()
                    .filter(|marker| {
                        marker
                            .preview_hit(
                                pointer_position,
                                plot_response,
                                calibration,
                                raw_axis_range,
                                log_x,
                                log_y,
                                bin_width,
                                &self.preview_background,
                            )
                            .is_some()
                    })
                    .min_by(|left, right| {
                        let x = plot_response
                            .transform
                            .value_from_position(pointer_position)
                            .x;
                        let left_distance =
                            raw_x_value(x, calibration, log_x, raw_axis_range, left.center.x_value)
                                .map_or(f64::INFINITY, |value| (value - left.center.x_value).abs());
                        let right_distance = raw_x_value(
                            x,
                            calibration,
                            log_x,
                            raw_axis_range,
                            right.center.x_value,
                        )
                        .map_or(f64::INFINITY, |value| (value - right.center.x_value).abs());
                        left_distance.total_cmp(&right_distance)
                    })
                    .map(|marker| marker.id)
            });
        for marker in &mut self.peak_markers {
            let interaction = marker.interactive_dragging(
                plot_response,
                calibration,
                raw_axis_range,
                log_x,
                log_y,
                bin_width,
                &self.preview_background,
                preview_owner,
            );
            changed |= interaction.changed;
            if interaction.center_moved {
                moved_peak_ids.push(marker.id);
            }
            shared_fwhm = shared_fwhm.or(interaction.shared_fwhm);
            shared_fwhm_bounds = shared_fwhm_bounds.or(interaction.shared_fwhm_bounds);
        }
        if equal_sigma && let Some(fwhm) = shared_fwhm {
            let common_minimum = self
                .peak_markers
                .iter()
                .map(|marker| marker.fwhm_min)
                .fold(f64::NEG_INFINITY, f64::max);
            let common_maximum = self
                .peak_markers
                .iter()
                .map(|marker| marker.fwhm_max)
                .fold(f64::INFINITY, f64::min);
            let fwhm = if common_minimum < common_maximum {
                fwhm.clamp(common_minimum, common_maximum)
            } else {
                fwhm
            };
            for marker in &mut self.peak_markers {
                marker.set_fwhm_preserving_height(fwhm, bin_width);
                marker.hold_current_seed();
                marker.valid =
                    marker.amplitude.is_finite() && marker.amplitude > 0.0 && marker.bounds_valid();
            }
        }
        if equal_sigma && let Some([minimum, maximum]) = shared_fwhm_bounds {
            for marker in &mut self.peak_markers {
                marker.fwhm_min = minimum;
                marker.fwhm_max = maximum;
                marker.hold_bounds();
                marker.valid =
                    marker.amplitude.is_finite() && marker.amplitude > 0.0 && marker.bounds_valid();
            }
        }
        if auto_estimate_moved_peak {
            for marker in &mut self.peak_markers {
                if moved_peak_ids.contains(&marker.id) {
                    marker.reset_estimates();
                }
            }
        }
        changed |= before_background != self.get_background_marker_positions();
        let regions_changed = before_regions != self.get_region_marker_positions();
        if regions_changed {
            self.reset_all_estimates();
        }
        changed |= regions_changed;
        if changed {
            self.peak_markers
                .sort_by(|left, right| left.center.x_value.total_cmp(&right.center.x_value));
            self.estimate_signature = 0;
        }
        changed
    }

    pub fn initial_guesses_ui(
        &mut self,
        ui: &mut egui::Ui,
        bin_width: f64,
        equal_sigma: bool,
    ) -> bool {
        if self.peak_markers.is_empty() {
            return false;
        }

        let mut changed = false;
        let mut reset_all = false;
        egui::CollapsingHeader::new("Initial Guesses")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(if equal_sigma {
                        "Shared FWHM: dragging or editing any width updates every peak."
                    } else {
                        "Independent FWHM: each peak width can be edited separately."
                    });
                    reset_all = ui.button("Reset All").clicked();
                });
                if let Some(error) = &self.estimate_error {
                    ui.colored_label(egui::Color32::YELLOW, error);
                }

                let mut shared_fwhm = None;
                let mut shared_fwhm_bounds = None;
                let mut reestimate = Vec::new();
                egui::ScrollArea::horizontal()
                    .id_salt("manual_peak_guess_scroll")
                    .show(ui, |ui| {
                        egui::Grid::new("manual_peak_guess_table")
                            .striped(true)
                            .show(ui, |ui| {
                                ui.strong("Peak");
                                ui.strong("Position min");
                                ui.strong("Position guess");
                                ui.strong("Position max");
                                ui.strong("FWHM min");
                                ui.strong("FWHM guess");
                                ui.strong("FWHM max");
                                ui.strong("Height min");
                                ui.strong("Height guess");
                                ui.strong("Height max");
                                ui.strong("Amplitude / area");
                                ui.strong("Source");
                                ui.strong("Valid");
                                ui.end_row();

                                for guess in &mut self.peak_markers {
                                    ui.label(format!("#{}", guess.id));
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut guess.center_min)
                                                .speed(bin_width * 0.1),
                                        )
                                        .changed()
                                    {
                                        guess.hold_current_seed();
                                        changed = true;
                                    }
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut guess.center.x_value)
                                                .speed(bin_width.max(1.0e-6) * 0.1),
                                        )
                                        .changed()
                                    {
                                        guess.hold_current_seed();
                                        changed = true;
                                    }
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut guess.center_max)
                                                .speed(bin_width * 0.1),
                                        )
                                        .changed()
                                    {
                                        guess.hold_current_seed();
                                        changed = true;
                                    }

                                    let mut width_bounds_changed = false;
                                    width_bounds_changed |= ui
                                        .add(
                                            egui::DragValue::new(&mut guess.fwhm_min)
                                                .speed(bin_width * 0.05),
                                        )
                                        .changed();
                                    let mut fwhm = guess.fwhm;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut fwhm)
                                                .speed(bin_width.max(1.0e-6) * 0.05)
                                                .range(
                                                    (0.5 * bin_width.max(1.0e-12))..=f64::INFINITY,
                                                ),
                                        )
                                        .changed()
                                    {
                                        guess.set_fwhm_preserving_height(fwhm, bin_width);
                                        guess.hold_current_seed();
                                        shared_fwhm = equal_sigma.then_some(guess.fwhm);
                                        changed = true;
                                    }
                                    width_bounds_changed |= ui
                                        .add(
                                            egui::DragValue::new(&mut guess.fwhm_max)
                                                .speed(bin_width * 0.05),
                                        )
                                        .changed();
                                    if width_bounds_changed {
                                        guess.hold_current_seed();
                                        shared_fwhm_bounds =
                                            equal_sigma.then_some([guess.fwhm_min, guess.fwhm_max]);
                                        changed = true;
                                    }

                                    let height_speed = guess.net_height.abs().max(1.0) * 0.01;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut guess.net_height_min)
                                                .speed(height_speed)
                                                .range(0.0..=f64::INFINITY),
                                        )
                                        .changed()
                                    {
                                        guess.hold_current_seed();
                                        changed = true;
                                    }
                                    let mut net_height = guess.net_height;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut net_height)
                                                .speed(height_speed)
                                                .range(0.0..=f64::INFINITY),
                                        )
                                        .changed()
                                    {
                                        guess.set_net_height(net_height, bin_width);
                                        guess.hold_current_seed();
                                        changed = true;
                                    }
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut guess.net_height_max)
                                                .speed(height_speed)
                                                .range(0.0..=f64::INFINITY),
                                        )
                                        .changed()
                                    {
                                        guess.hold_current_seed();
                                        changed = true;
                                    }
                                    ui.label(format!(
                                        "{:.5} / {:.5}",
                                        guess.amplitude,
                                        guess.amplitude / bin_width
                                    ));
                                    ui.label(format!(
                                        "W: {} · H: {} · B: {}",
                                        guess.width_source.label(),
                                        guess.amplitude_source.label(),
                                        guess.bounds_source.label(),
                                    ));
                                    guess.valid = guess.center.x_value.is_finite()
                                        && guess.fwhm.is_finite()
                                        && guess.fwhm > 0.0
                                        && guess.amplitude.is_finite()
                                        && guess.amplitude > 0.0
                                        && guess.bounds_valid();
                                    ui.colored_label(
                                        if guess.valid {
                                            egui::Color32::GREEN
                                        } else {
                                            egui::Color32::RED
                                        },
                                        if guess.valid { "Yes" } else { "No" },
                                    );
                                    if ui.button("Re-estimate").clicked() {
                                        reestimate.push(guess.id);
                                    }
                                    ui.end_row();
                                }
                            });
                    });

                if let Some(fwhm) = shared_fwhm {
                    for guess in &mut self.peak_markers {
                        guess.set_fwhm_preserving_height(fwhm, bin_width);
                        guess.hold_current_seed();
                    }
                }
                if let Some([minimum, maximum]) = shared_fwhm_bounds {
                    for guess in &mut self.peak_markers {
                        guess.fwhm_min = minimum;
                        guess.fwhm_max = maximum;
                        guess.hold_current_seed();
                    }
                }
                if !reestimate.is_empty() {
                    for guess in &mut self.peak_markers {
                        if reestimate.contains(&guess.id) {
                            guess.reset_estimates();
                        }
                    }
                    changed = true;
                }
            });

        if reset_all {
            for guess in &mut self.peak_markers {
                guess.reset_estimates();
            }
            changed = true;
        }
        if changed {
            self.peak_markers
                .sort_by(|left, right| left.center.x_value.total_cmp(&right.center.x_value));
            self.estimate_signature = 0;
        }
        changed
    }

    pub fn menu_button(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.manual_marker_position)
                    .speed(1.0)
                    .prefix("Marker Position: "),
            );

            ui.horizontal(|ui| {
                if ui.button("Peak").clicked() {
                    self.add_peak_marker(self.manual_marker_position);
                }

                ui.separator();

                if ui.button("Background").clicked() {
                    self.add_background_pair(self.manual_marker_position, 1.0);
                }

                ui.separator();

                if ui.button("Region").clicked() {
                    if self.region_markers.len() > 1 {
                        self.clear_region_markers();
                    }
                    self.add_region_marker(self.manual_marker_position);
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Clear");

                if ui.button("All").clicked() {
                    self.clear_background_markers();
                    self.clear_peak_markers();
                    self.clear_region_markers();
                }

                if ui.button("Region").clicked() {
                    self.clear_region_markers();
                }

                if ui.button("Peaks").clicked() {
                    self.clear_peak_markers();
                }

                if ui.button("Background").clicked() {
                    self.clear_background_markers();
                }
            });
        });

        // ui.separator();

        // egui::ScrollArea::vertical().show(ui, |ui| {
        //     for marker in &mut self.region_markers {
        //         marker.menu_button(ui);
        //     }

        //     for marker in &mut self.peak_markers {
        //         marker.menu_button(ui);
        //     }

        //     for pair in &mut self.background_markers {
        //         pair.start.menu_button(ui);
        //         pair.end.menu_button(ui);
        //         pair.histogram_line.menu_button(ui);
        //     }
        // });
    }
}

#[cfg(test)]
mod tests {
    use super::{FitMarkers, GuessSource, PeakGuess};
    use crate::egui_plot_stuff::egui_vertical_line::EguiVerticalLine;

    fn guess() -> PeakGuess {
        let mut guess = PeakGuess {
            id: 7,
            center: EguiVerticalLine::new(10.0, egui::Color32::RED),
            fwhm: 6.0,
            amplitude: 1_000.0,
            center_min: 8.0,
            center_max: 12.0,
            fwhm_min: 4.0,
            fwhm_max: 12.0,
            net_height_min: 50.0,
            net_height_max: 200.0,
            width_source: GuessSource::Manual,
            amplitude_source: GuessSource::Manual,
            bounds_source: GuessSource::Manual,
            ..PeakGuess::default()
        };
        guess.net_height = guess.model_height(1.0);
        guess.net_height_min = 0.5 * guess.net_height;
        guess.net_height_max = 1.5 * guess.net_height;
        guess
    }

    #[test]
    fn width_edit_holds_center_and_net_height_constant() {
        let mut guess = guess();
        let center = guess.center.x_value;
        let height = guess.net_height;
        assert!(guess.set_fwhm_preserving_height(10.0, 1.0));
        assert_eq!(guess.center.x_value, center);
        assert!((guess.net_height - height).abs() < 1.0e-10);
        assert_eq!(guess.fwhm, 10.0);
    }

    #[test]
    fn height_edit_holds_center_and_fwhm_constant() {
        let mut guess = guess();
        let center = guess.center.x_value;
        let fwhm = guess.fwhm;
        assert!(guess.set_net_height(120.0, 1.0));
        assert_eq!(guess.center.x_value, center);
        assert_eq!(guess.fwhm, fwhm);
        assert!((guess.model_height(1.0) - 120.0).abs() < 1.0e-10);
    }

    #[test]
    fn stable_peak_ids_survive_sorting_and_deletion() {
        let mut markers = FitMarkers::new();
        markers.add_peak_marker(8.0);
        markers.add_peak_marker(2.0);
        let ids = markers
            .peak_markers
            .iter()
            .map(|guess| guess.id)
            .collect::<Vec<_>>();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[0], ids[1]);
        let retained_id = markers.peak_markers[1].id;
        markers.delete_closest_marker(2.0);
        assert_eq!(markers.peak_markers.len(), 1);
        assert_eq!(markers.peak_markers[0].id, retained_id);
    }

    #[test]
    fn removing_a_peak_reestimates_the_remaining_peaks() {
        let mut markers = FitMarkers::new();
        let mut first = guess();
        first.id = 1;
        let mut second = guess();
        second.id = 2;
        second.center.x_value = 20.0;
        markers.peak_markers = vec![first, second];

        markers.delete_closest_marker(10.0);

        assert_eq!(markers.peak_markers.len(), 1);
        let remaining = &markers.peak_markers[0];
        assert_eq!(remaining.width_source, GuessSource::Estimated);
        assert_eq!(remaining.amplitude_source, GuessSource::Estimated);
        assert_eq!(remaining.bounds_source, GuessSource::Estimated);
        assert!(!remaining.valid);
        assert_eq!(markers.estimate_signature, 0);
    }

    #[test]
    fn legacy_plain_peak_line_loads_as_an_estimated_guess() {
        let mut serialized = serde_json::to_value(FitMarkers::new()).expect("markers serialize");
        serialized["peak_markers"] =
            serde_json::json!([EguiVerticalLine::new(42.25, egui::Color32::RED)]);
        let mut restored: FitMarkers =
            serde_json::from_value(serialized).expect("legacy marker state loads");
        restored.ensure_peak_ids();
        assert_eq!(restored.peak_markers.len(), 1);
        assert_eq!(restored.peak_markers[0].center.x_value, 42.25);
        assert_eq!(
            restored.peak_markers[0].width_source,
            GuessSource::Estimated
        );
        assert_ne!(restored.peak_markers[0].id, 0);
    }

    #[test]
    fn peak_guess_round_trip_preserves_saved_values_and_resets_ui_state() {
        let mut markers = FitMarkers::new();
        let mut saved_guess = guess();
        saved_guess.valid = true;
        saved_guess.clean_width = true;
        saved_guess.preview_hovered = true;
        saved_guess.preview_upper_bounds_hovered = true;
        saved_guess.preview_lower_bounds_hovered = true;
        saved_guess.seed_drag_start = Some(egui::pos2(12.0, 34.0));
        saved_guess.seed_drag_vertical = Some(true);
        saved_guess.preview_position_hovered = true;
        markers.peak_markers = vec![saved_guess];

        let serialized = serde_json::to_value(&markers).expect("markers serialize");
        let restored: FitMarkers = serde_json::from_value(serialized).expect("markers load");
        let guess = &restored.peak_markers[0];

        assert_eq!(guess.id, 7);
        assert_eq!(guess.center.x_value, 10.0);
        assert_eq!(guess.fwhm, 6.0);
        assert_eq!(guess.amplitude, 1_000.0);
        assert!(guess.net_height.is_finite() && guess.net_height > 0.0);
        assert_eq!(guess.center_min, 8.0);
        assert_eq!(guess.center_max, 12.0);
        assert_eq!(guess.fwhm_min, 4.0);
        assert_eq!(guess.fwhm_max, 12.0);
        assert_eq!(guess.net_height_min, 0.5 * guess.net_height);
        assert_eq!(guess.net_height_max, 1.5 * guess.net_height);
        assert_eq!(guess.width_source, GuessSource::Manual);
        assert_eq!(guess.amplitude_source, GuessSource::Manual);
        assert_eq!(guess.bounds_source, GuessSource::Manual);
        assert!(guess.valid);
        assert!(guess.clean_width);
        assert!(!guess.preview_hovered);
        assert!(!guess.preview_upper_bounds_hovered);
        assert!(!guess.preview_lower_bounds_hovered);
        assert_eq!(guess.seed_drag_start, None);
        assert_eq!(guess.seed_drag_vertical, None);
        assert!(!guess.preview_position_hovered);
    }
}
