#[derive(PartialEq, Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Data {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Value {
    pub value: f64,
    pub uncertainty: f64,
}

impl Value {
    pub fn ui(&mut self, ui: &mut egui::Ui, name: Option<&str>) {
        ui.horizontal(|ui| {
            if let Some(name) = name {
                ui.label(name);
            }
            ui.add(egui::DragValue::new(&mut self.value).speed(0.1))
                .on_hover_text("Value of the parameter");

            ui.label("±");

            ui.add(egui::DragValue::new(&mut self.uncertainty).speed(0.1))
                .on_hover_text("Uncertainty of the parameter");
        });
    }
}

#[derive(PartialEq, Debug, Clone, serde::Deserialize, serde::Serialize)]
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
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.a.ui(ui, Some("a:"));
            ui.separator();
            self.b.ui(ui, Some("b:"));
            ui.separator();
            self.c.ui(ui, Some("c:"));
        });
    }

    pub fn calibrate(&self, x: f64) -> f64 {
        self.a.value * x * x + self.b.value * x + self.c.value
    }

    pub fn invert(&self, energy: f64) -> Option<f64> {
        let a = self.a.value;
        let b = self.b.value;
        let c = self.c.value;

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
pub struct Parameter {
    pub name: String,
    pub min: f64,
    pub max: f64,
    pub initial_guess: f64,
    pub vary: bool,
    pub value: Option<f64>,
    pub uncertainty: Option<f64>,
    pub calibrated_value: Option<f64>,
    pub calibrated_uncertainty: Option<f64>,
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
        if let Some(x) = self.value {
            let dx = self.uncertainty.unwrap_or(0.0);

            let a = calibration.a.value;
            let b = calibration.b.value;
            let c = calibration.c.value;

            let da = calibration.a.uncertainty;
            let db = calibration.b.uncertainty;
            let dc = calibration.c.uncertainty;

            let energy = a * x * x + b * x + c;

            // let da_term = a * x * x * ((da / a).powi(2) + 2.0 * (dx / x).powi(2)).sqrt();
            // let db_term = b * x * (db / b).hypot(dx / x);
            // let dc_term = dc;

            // let de = (da_term.powi(2) + db_term.powi(2) + dc_term.powi(2)).sqrt();

            let j0 = x.powi(2);
            let j1 = x;
            let j2 = 1.0;

            let sigma_params_sq = if let Some(cov) = &calibration.cov {
                println!("Using full covariance matrix for uncertainty propagation");
                // J Σ J^T
                let t0 = j0 * (cov[0][0] * j0 + cov[0][1] * j1 + cov[0][2] * j2);
                let t1 = j1 * (cov[1][0] * j0 + cov[1][1] * j1 + cov[1][2] * j2);
                let t2 = j2 * (cov[2][0] * j0 + cov[2][1] * j1 + cov[2][2] * j2);
                t0 + t1 + t2
            } else {
                // Fallback: assume independence (no covariances)
                (j0 * da).powi(2) + (j1 * db).powi(2) + (j2 * dc).powi(2)
            };

            // x uncertainty (assumed independent of {a,b,c})
            let dy_dx = 2.0 * a * x + b;
            let sigma_x_sq = (dy_dx * dx).powi(2);

            let de = (sigma_params_sq + sigma_x_sq).sqrt();

            self.calibrated_value = Some(energy);
            self.calibrated_uncertainty = Some(de);
        } else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
        }
    }

    pub fn calibrate_sigma(&mut self, calibration: &Calibration, x: f64) {
        if let Some(sigma_x) = self.value {
            let a = calibration.a.value;
            let b = calibration.b.value;

            let da = calibration.a.uncertainty;
            let db = calibration.b.uncertainty;

            let dedx = 2.0 * a * x + b;
            let dedx_unc = (2.0 * x * da).hypot(db);

            let sigma_e = dedx.abs() * sigma_x;
            let dsigma_e = (dedx * self.uncertainty.unwrap_or(0.0)).hypot(sigma_x * dedx_unc);

            self.calibrated_value = Some(sigma_e);
            self.calibrated_uncertainty = Some(dsigma_e);
        } else {
            self.calibrated_value = None;
            self.calibrated_uncertainty = None;
        }
    }

    pub fn calibrate_fwhm(&mut self, calibration: &Calibration, x: f64) {
        if let Some(fwhm_x) = self.value {
            let sigma_x = fwhm_x / 2.355;
            let a = calibration.a.value;
            let b = calibration.b.value;

            let da = calibration.a.uncertainty;
            let db = calibration.b.uncertainty;

            let dedx = 2.0 * a * x + b;
            let dedx_unc = (2.0 * x * da).hypot(db);

            let fwhm_e = dedx.abs() * sigma_x * 2.355;
            let dfwhm_e =
                (dedx * self.uncertainty.unwrap_or(0.0)).hypot(sigma_x * 2.355 * dedx_unc);

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
