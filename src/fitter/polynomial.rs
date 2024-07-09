use crate::egui_plot_stuff::egui_line::EguiLine;
use compute::predict::PolynomialRegressor;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PolynomialFitter {
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub degree: usize,
    pub coefficients: Option<Vec<f64>>,
    pub fit_line: EguiLine,
}

impl PolynomialFitter {
    /// Creates a new PolynomialFitter with the given data.
    pub fn new(degree: usize) -> Self {
        let mut fit_line = EguiLine::new(egui::Color32::GREEN);
        fit_line.name = "Polynomial Fit".to_string();

        PolynomialFitter {
            x_data: Vec::new(),
            y_data: Vec::new(),
            degree,
            coefficients: None,
            fit_line,
        }
    }

    pub fn fit(&mut self) {
        let mut regressor = PolynomialRegressor::new(self.degree);

        if self.x_data.len() < self.degree + 1 {
            log::error!(
                "Not enough data points to fit polynomial of degree {}",
                self.degree
            );
            return;
        }
        regressor.fit(&self.x_data, &self.y_data);

        self.coefficients = Some(regressor.coef.clone());
        self.compute_fit_points();

        log::info!("Polynomial fit coefficients: {:?}", regressor.coef);
    }

    pub fn subtract_background(&self, x_data: Vec<f64>, y_data: Vec<f64>) -> Vec<f64> {
        if let Some(coef) = &self.coefficients {
            if coef.is_empty() {
                log::error!("No coefficients found for polynomial fit");
                return y_data;
            }

            let mut y_data = y_data.clone();

            for (i, x) in x_data.iter().enumerate() {
                let y = coef
                    .iter()
                    .enumerate()
                    .fold(0.0, |acc, (j, c)| acc + c * x.powi(j as i32));
                y_data[i] -= y;
            }

            y_data
        } else {
            y_data
        }
    }

    fn compute_fit_points(&mut self) {
        if let Some(coef) = &self.coefficients {
            if coef.is_empty() {
                log::error!("No coefficients found for polynomial fit");
                return;
            }

            self.fit_line.clear_points();

            // get the min and max x values
            let (x_min, x_max) = self
                .x_data
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &x| {
                    (min.min(x), max.max(x))
                });

            let number_points = 1000;
            for i in 0..number_points {
                let x = x_min + (x_max - x_min) / (number_points as f64) * (i as f64);
                let y = coef
                    .iter()
                    .enumerate()
                    .fold(0.0, |acc, (j, c)| acc + c * x.powi(j as i32));
                self.fit_line.add_point(x, y);
            }
        }
    }

    pub fn _draw(&self, plot_ui: &mut egui_plot::PlotUi) {
        self.fit_line.draw(plot_ui);
    }

    pub fn fit_params_ui(&self, ui: &mut egui::Ui) {
        // ui.horizontal(|ui| {
        //     ui.label("Polynomial degree:");
        //     ui.add(egui::DragValue::new(&mut self.degree).speed(1.0));
        // });

        // if ui.button("Fit").clicked() {
        //     self.fit();
        // }

        ui.label("Coefficients:");
        if let Some(coef) = &self.coefficients {
            if coef.is_empty() {
                ui.label("No coefficients found");
            } else {
                for (i, coef) in coef.iter().enumerate() {
                    ui.label(format!("c{}: {}", i, coef));
                }
            }
        } else {
            ui.label("No coefficients found");
        }
    }
}
