use serde::de::{self, Visitor};
use std::fmt;

#[derive(PartialEq, Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Data {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

fn deserialize_f64_with_default<'de, D>(deserializer: D, default: f64) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct F64OrNullVisitor {
        default: f64,
    }

    impl<'de> Visitor<'de> for F64OrNullVisitor {
        type Value = f64;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a float, optional float, or null")
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value)
        }

        fn visit_f32<E>(self, value: f32) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(f64::from(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as f64)
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(value as f64)
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(self.default)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(self.default)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserialize_f64_with_default(deserializer, self.default)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let value = value.trim();
            let lower = value.to_ascii_lowercase();
            match lower.as_str() {
                "inf" | "+inf" | "infinity" | "+infinity" => Ok(f64::INFINITY),
                "-inf" | "-infinity" => Ok(f64::NEG_INFINITY),
                "nan" => Ok(f64::NAN),
                _ => value.parse::<f64>().map_err(E::custom),
            }
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(F64OrNullVisitor { default })
}

fn deserialize_f64_or_default<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_f64_with_default(deserializer, 0.0)
}

fn deserialize_f64_or_neg_infinity<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_f64_with_default(deserializer, f64::NEG_INFINITY)
}

fn deserialize_f64_or_pos_infinity<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    deserialize_f64_with_default(deserializer, f64::INFINITY)
}

#[derive(PartialEq, Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Value {
    #[serde(default, deserialize_with = "deserialize_f64_or_default")]
    pub value: f64,
    #[serde(
        default,
        alias = "uncertainity",
        deserialize_with = "deserialize_f64_or_default"
    )]
    pub uncertainty: f64,
}

impl Value {
    pub fn ui(&mut self, ui: &mut egui::Ui, name: Option<&str>) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            if let Some(name) = name {
                ui.label(name);
            }
            changed |= ui
                .add(egui::DragValue::new(&mut self.value).speed(0.1))
                .on_hover_text("Value of the parameter")
                .changed();

            ui.label("±");

            changed |= ui
                .add(egui::DragValue::new(&mut self.uncertainty).speed(0.1))
                .on_hover_text("Uncertainty of the parameter")
                .changed();
        });
        changed
    }
}

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Calibration {
    pub a: Value,
    pub b: Value,
    pub c: Value,
    pub cov: Option<[[f64; 3]; 3]>,
}

impl Default for Calibration {
    fn default() -> Self {
        Self {
            a: Value {
                value: 0.0,
                uncertainty: 0.0,
            },
            b: Value {
                value: 1.0,
                uncertainty: 0.0,
            },
            c: Value {
                value: 0.0,
                uncertainty: 0.0,
            },
            cov: None,
        }
    }
}

impl Calibration {
    const INVERT_EPSILON: f64 = 1e-12;
    const INTERPOLATION_SAMPLES: usize = 2049;

    pub fn ui(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            changed |= self.a.ui(ui, Some("a:"));
            ui.separator();
            changed |= self.b.ui(ui, Some("b:"));
            ui.separator();
            changed |= self.c.ui(ui, Some("c:"));
        });
        changed
    }

    pub fn calibrate(&self, x: f64) -> f64 {
        self.a.value * x * x + self.b.value * x + self.c.value
    }

    pub fn coefficients_are_finite(&self) -> bool {
        self.a.value.is_finite()
            && self.a.uncertainty.is_finite()
            && self.b.value.is_finite()
            && self.b.uncertainty.is_finite()
            && self.c.value.is_finite()
            && self.c.uncertainty.is_finite()
            && self
                .cov
                .iter()
                .flatten()
                .flatten()
                .all(|value| value.is_finite())
    }

    pub fn calibrate_checked(&self, x: f64) -> Option<f64> {
        if !self.coefficients_are_finite() || !x.is_finite() {
            return None;
        }

        let energy = self.calibrate(x);
        energy.is_finite().then_some(energy)
    }

    pub fn derivative(&self, x: f64) -> f64 {
        2.0 * self.a.value * x + self.b.value
    }

    pub fn derivative_checked(&self, x: f64) -> Option<f64> {
        if !self.coefficients_are_finite() || !x.is_finite() {
            return None;
        }

        let derivative = self.derivative(x);
        derivative.is_finite().then_some(derivative)
    }

    pub fn curve_uncertainty(&self, x: f64) -> f64 {
        let j0 = x.powi(2);
        let j1 = x;
        let j2 = 1.0;

        let variance = if let Some(cov) = &self.cov {
            let t0 = j0 * (cov[0][0] * j0 + cov[0][1] * j1 + cov[0][2] * j2);
            let t1 = j1 * (cov[1][0] * j0 + cov[1][1] * j1 + cov[1][2] * j2);
            let t2 = j2 * (cov[2][0] * j0 + cov[2][1] * j1 + cov[2][2] * j2);
            t0 + t1 + t2
        } else {
            (j0 * self.a.uncertainty).powi(2)
                + (j1 * self.b.uncertainty).powi(2)
                + (j2 * self.c.uncertainty).powi(2)
        };

        variance.max(0.0).sqrt()
    }

    pub fn curve_uncertainty_checked(&self, x: f64) -> Option<f64> {
        if !self.coefficients_are_finite() || !x.is_finite() {
            return None;
        }

        let uncertainty = self.curve_uncertainty(x);
        uncertainty.is_finite().then_some(uncertainty)
    }

    fn sorted_range(range: (f64, f64)) -> Option<(f64, f64)> {
        if !range.0.is_finite() || !range.1.is_finite() {
            return None;
        }

        Some(if range.0 <= range.1 {
            range
        } else {
            (range.1, range.0)
        })
    }

    fn sampled_curve(&self, raw_range: (f64, f64)) -> Option<Vec<(f64, f64)>> {
        let (raw_min, raw_max) = Self::sorted_range(raw_range)?;
        if !self.coefficients_are_finite() {
            return None;
        }

        let sample_count = Self::INTERPOLATION_SAMPLES.max(2);
        let step = if sample_count > 1 {
            (raw_max - raw_min) / (sample_count.saturating_sub(1) as f64)
        } else {
            0.0
        };

        let mut samples = Vec::with_capacity(sample_count);
        for index in 0..sample_count {
            let raw = if index + 1 == sample_count {
                raw_max
            } else {
                raw_min + step * index as f64
            };
            let energy = self.calibrate_checked(raw)?;
            samples.push((raw, energy));
        }

        Some(samples)
    }

    fn interpolation_tolerances(samples: &[(f64, f64)]) -> (f64, f64) {
        let raw_min = samples
            .iter()
            .map(|(raw, _)| *raw)
            .fold(f64::INFINITY, f64::min);
        let raw_max = samples
            .iter()
            .map(|(raw, _)| *raw)
            .fold(f64::NEG_INFINITY, f64::max);
        let energy_min = samples
            .iter()
            .map(|(_, energy)| *energy)
            .fold(f64::INFINITY, f64::min);
        let energy_max = samples
            .iter()
            .map(|(_, energy)| *energy)
            .fold(f64::NEG_INFINITY, f64::max);

        let raw_tolerance = (raw_max - raw_min).abs() * 1e-9 + Self::INVERT_EPSILON;
        let energy_tolerance = (energy_max - energy_min).abs() * 1e-9 + Self::INVERT_EPSILON;
        (raw_tolerance, energy_tolerance)
    }

    pub fn turning_point(&self) -> Option<f64> {
        if !self.coefficients_are_finite() || self.a.value.abs() < Self::INVERT_EPSILON {
            return None;
        }

        let turning_point = -self.b.value / (2.0 * self.a.value);
        turning_point.is_finite().then_some(turning_point)
    }

    pub fn is_monotonic_on(&self, raw_range: (f64, f64)) -> bool {
        let Some(samples) = self.sampled_curve(raw_range) else {
            return false;
        };

        let (_, energy_tolerance) = Self::interpolation_tolerances(&samples);
        let mut saw_rising = false;
        let mut saw_falling = false;

        for window in samples.windows(2) {
            let delta = window[1].1 - window[0].1;
            if delta > energy_tolerance {
                saw_rising = true;
            } else if delta < -energy_tolerance {
                saw_falling = true;
            }

            if saw_rising && saw_falling {
                return false;
            }
        }

        saw_rising || saw_falling
    }

    pub fn display_bounds_for_raw_range(&self, raw_range: (f64, f64)) -> Option<(f64, f64)> {
        let samples = self.sampled_curve(raw_range)?;
        let display_min = samples
            .iter()
            .map(|(_, energy)| *energy)
            .fold(f64::INFINITY, f64::min);
        let display_max = samples
            .iter()
            .map(|(_, energy)| *energy)
            .fold(f64::NEG_INFINITY, f64::max);

        (display_min.is_finite() && display_max.is_finite()).then_some((display_min, display_max))
    }

    pub fn can_display_on(&self, raw_range: (f64, f64)) -> bool {
        self.display_bounds_for_raw_range(raw_range).is_some()
    }

    pub fn is_display_safe_on(&self, raw_range: (f64, f64)) -> bool {
        self.is_monotonic_on(raw_range) && self.display_bounds_for_raw_range(raw_range).is_some()
    }

    fn nearest_sample_raw_for_energy(
        energy: f64,
        samples: &[(f64, f64)],
        hint_raw: Option<f64>,
    ) -> Option<f64> {
        let hint_raw = hint_raw.filter(|value| value.is_finite());
        samples
            .iter()
            .min_by(|(raw_a, energy_a), (raw_b, energy_b)| {
                let energy_cmp = (energy - *energy_a)
                    .abs()
                    .total_cmp(&(energy - *energy_b).abs());
                if energy_cmp != std::cmp::Ordering::Equal {
                    energy_cmp
                } else if let Some(hint_raw) = hint_raw {
                    (hint_raw - *raw_a)
                        .abs()
                        .total_cmp(&(hint_raw - *raw_b).abs())
                } else {
                    raw_a.total_cmp(raw_b)
                }
            })
            .map(|(raw, _)| *raw)
    }

    pub fn invert_in_range_with_hint(
        &self,
        energy: f64,
        raw_range: (f64, f64),
        hint_raw: Option<f64>,
    ) -> Option<f64> {
        let samples = self.sampled_curve(raw_range)?;

        if !energy.is_finite() {
            return None;
        }

        let (raw_tolerance, energy_tolerance) = Self::interpolation_tolerances(&samples);
        let mut candidates = Vec::new();

        for window in samples.windows(2) {
            let (raw_0, energy_0) = window[0];
            let (raw_1, energy_1) = window[1];
            let segment_min = energy_0.min(energy_1) - energy_tolerance;
            let segment_max = energy_0.max(energy_1) + energy_tolerance;
            if energy < segment_min || energy > segment_max {
                continue;
            }

            let delta_energy = energy_1 - energy_0;
            let candidate = if delta_energy.abs() <= energy_tolerance {
                if let Some(hint_raw) = hint_raw.filter(|hint| {
                    *hint >= raw_0.min(raw_1) - raw_tolerance
                        && *hint <= raw_0.max(raw_1) + raw_tolerance
                }) {
                    hint_raw
                } else {
                    (raw_0 + raw_1) * 0.5
                }
            } else {
                let t = ((energy - energy_0) / delta_energy).clamp(0.0, 1.0);
                raw_0 + t * (raw_1 - raw_0)
            };

            if candidate.is_finite() {
                candidates.push(candidate);
            }
        }

        candidates.sort_by(|left, right| left.total_cmp(right));
        candidates.dedup_by(|left, right| (*left - *right).abs() <= raw_tolerance);

        if candidates.is_empty() {
            return Self::nearest_sample_raw_for_energy(energy, &samples, hint_raw);
        }

        hint_raw
            .filter(|hint| hint.is_finite())
            .and_then(|hint| {
                candidates
                    .iter()
                    .min_by(|left, right| (hint - **left).abs().total_cmp(&(hint - **right).abs()))
                    .copied()
            })
            .or_else(|| (candidates.len() == 1).then_some(candidates[0]))
            .or_else(|| Self::nearest_sample_raw_for_energy(energy, &samples, hint_raw))
    }

    pub fn invert_in_range(&self, energy: f64, raw_range: (f64, f64)) -> Option<f64> {
        self.invert_in_range_with_hint(energy, raw_range, None)
    }

    pub fn invert(&self, energy: f64) -> Option<f64> {
        let a = self.a.value;
        let b = self.b.value;
        let c = self.c.value;

        if !energy.is_finite() || !self.coefficients_are_finite() {
            return None;
        }

        if a.abs() < 1e-12 {
            // Linear case: E = bx + c ⇒ x = (E - c)/b
            if b.abs() < 1e-12 {
                return None; // Not invertible
            }
            return Some((energy - c) / b);
        }

        // Quadratic case: E = ax² + bx + c ⇒ solve ax² + bx + (c - E) = 0
        let discriminant = b * b - 4.0 * a * (c - energy);

        if discriminant < 0.0 {
            return None; // No real roots
        }

        let sqrt_disc = discriminant.sqrt();

        // Return the root closer to 0 (can adjust this if needed)
        let x1 = (-b + sqrt_disc) / (2.0 * a);
        let x2 = (-b - sqrt_disc) / (2.0 * a);

        // Choose the root that's in a reasonable range
        Some(if x1.abs() < x2.abs() { x1 } else { x2 })
    }
}

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Parameter {
    pub name: String,
    #[serde(
        default = "default_parameter_min",
        deserialize_with = "deserialize_f64_or_neg_infinity"
    )]
    pub min: f64,
    #[serde(
        default = "default_parameter_max",
        deserialize_with = "deserialize_f64_or_pos_infinity"
    )]
    pub max: f64,
    #[serde(default, deserialize_with = "deserialize_f64_or_default")]
    pub initial_guess: f64,
    pub vary: bool,
    pub value: Option<f64>,
    pub uncertainty: Option<f64>,
    pub calibrated_value: Option<f64>,
    pub calibrated_uncertainty: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::{Parameter, Value};

    #[test]
    fn parameter_deserializes_null_min_max_as_infinities() {
        let json = r#"{
            "name":"slope",
            "min":null,
            "max":null,
            "initial_guess":0.0,
            "vary":true,
            "value":null,
            "uncertainty":null,
            "calibrated_value":null,
            "calibrated_uncertainty":null
        }"#;

        let p: Parameter = serde_json::from_str(json).expect("parameter should deserialize");
        assert!(p.min.is_infinite() && p.min.is_sign_negative());
        assert!(p.max.is_infinite() && p.max.is_sign_positive());
    }

    #[test]
    fn value_deserializes_legacy_uncertainity_field() {
        let json = r#"{"value": 42.0, "uncertainity": 0.5}"#;
        let v: Value = serde_json::from_str(json).expect("value should deserialize");
        assert_eq!(v.value, 42.0);
        assert_eq!(v.uncertainty, 0.5);
    }

    #[test]
    fn parameter_deserializes_ron_infinite_bounds() {
        let ron = r#"(name:"slope",min:-inf,max:inf,initial_guess:0.0,vary:true,value:None,uncertainty:None,calibrated_value:None,calibrated_uncertainty:None)"#;
        let p: Parameter = ron::from_str(ron).expect("parameter should deserialize from ron");
        assert!(p.min.is_infinite() && p.min.is_sign_negative());
        assert!(p.max.is_infinite() && p.max.is_sign_positive());
    }
}

fn default_parameter_min() -> f64 {
    f64::NEG_INFINITY
}

fn default_parameter_max() -> f64 {
    f64::INFINITY
}

impl Default for Parameter {
    fn default() -> Self {
        Self {
            name: String::new(),
            min: f64::NEG_INFINITY,
            max: f64::INFINITY,
            initial_guess: 0.0,
            vary: true,
            value: None,
            uncertainty: None,
            calibrated_value: None,
            calibrated_uncertainty: None,
        }
    }
}

impl Parameter {
    pub fn calibrate_energy(&mut self, calibration: &Calibration) {
        let Some(x) = self.value.filter(|value| value.is_finite()) else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
            return;
        };

        let dx = self.uncertainty.unwrap_or(0.0);
        let Some(energy) = calibration.calibrate_checked(x) else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
            return;
        };
        let Some(curve_uncertainty) = calibration.curve_uncertainty_checked(x) else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
            return;
        };
        let Some(dy_dx) = calibration.derivative_checked(x) else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
            return;
        };

        let sigma_params_sq = curve_uncertainty.powi(2);
        let sigma_x_sq = (dy_dx * dx).powi(2);
        let de = (sigma_params_sq + sigma_x_sq).sqrt();

        if de.is_finite() {
            self.calibrated_value = Some(energy);
            self.calibrated_uncertainty = Some(de);
        } else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
        }
    }

    pub fn calibrate_sigma(&mut self, calibration: &Calibration, x: f64) {
        let Some(sigma_x) = self.value.filter(|value| value.is_finite()) else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
            return;
        };

        let Some(dedx) = calibration.derivative_checked(x) else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
            return;
        };

        let da = calibration.a.uncertainty;
        let db = calibration.b.uncertainty;
        let dedx_unc = (2.0 * x * da).hypot(db);

        let sigma_e = dedx.abs() * sigma_x;
        let dsigma_e = (dedx * self.uncertainty.unwrap_or(0.0)).hypot(sigma_x * dedx_unc);

        if sigma_e.is_finite() && dsigma_e.is_finite() {
            self.calibrated_value = Some(sigma_e);
            self.calibrated_uncertainty = Some(dsigma_e);
        } else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
        }
    }

    pub fn calibrate_fwhm(&mut self, calibration: &Calibration, x: f64) {
        let Some(fwhm_x) = self.value.filter(|value| value.is_finite()) else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
            return;
        };

        let Some(dedx) = calibration.derivative_checked(x) else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
            return;
        };

        let sigma_x = fwhm_x / 2.355;
        let da = calibration.a.uncertainty;
        let db = calibration.b.uncertainty;
        let dedx_unc = (2.0 * x * da).hypot(db);

        let fwhm_e = dedx.abs() * sigma_x * 2.355;
        let dfwhm_e = (dedx * self.uncertainty.unwrap_or(0.0)).hypot(sigma_x * 2.355 * dedx_unc);

        if fwhm_e.is_finite() && dfwhm_e.is_finite() {
            self.calibrated_value = Some(fwhm_e);
            self.calibrated_uncertainty = Some(dfwhm_e);
        } else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label(&self.name);
        ui.add(
            egui::DragValue::new(&mut self.initial_guess).speed(0.1), // .prefix("Initial Guess: ")
                                                                      // .suffix(" a.u."),
        )
        .on_hover_text(format!("Initial guess for the {} parameter", self.name));

        ui.add(
            egui::DragValue::new(&mut self.min)
                .speed(0.1)
                // .prefix("Min: ")
                .range(f64::NEG_INFINITY..=self.max), // .suffix(" a.u."),
        )
        .on_hover_text(format!("Minimum value for the {} parameter", self.name));

        ui.add(
            egui::DragValue::new(&mut self.max)
                .speed(0.1)
                // .prefix("Max: ")
                .range(self.min..=f64::INFINITY), // .suffix(" a.u."),
        )
        .on_hover_text(format!("Maximum value for the {} parameter", self.name));

        ui.checkbox(&mut self.vary, "").on_hover_text(format!(
            "Allow the {} parameter to vary during the fitting process",
            self.name
        ));

        if let Some(value) = self.value {
            ui.separator();
            ui.label(format!("{value:.3}"));
            ui.label(format!("{:.3}", self.uncertainty.unwrap_or(0.0)));
        }
    }
}
