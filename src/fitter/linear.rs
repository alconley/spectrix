#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Parameters {
    pub slope: f64,
    pub intercept: f64,
}


#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct LinearFitter {
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    fit_params: Option<Parameters>,
    fit_line: Option<Vec<(f64, f64)>>
}

impl LinearFitter {
    pub fn new(x_data: Vec<f64>, y_data: Vec<f64>) -> Self {
        LinearFitter {
            x_data,
            y_data,
            fit_params: None,
            fit_line: None,
        }
    }

    pub fn simple_linear_regression(
        x_data: &[f64],
        y_data: &[f64],
    ) -> Result<(f64, f64), &'static str> {
        if x_data.len() != y_data.len() || x_data.is_empty() {
            return Err("x_data and y_data must have the same non-zero length");
        }

        let n = x_data.len() as f64;
        let sum_x = x_data.iter().sum::<f64>();
        let sum_y = y_data.iter().sum::<f64>();
        let sum_xy = x_data
            .iter()
            .zip(y_data.iter())
            .map(|(x, y)| x * y)
            .sum::<f64>();
        let sum_x_squared = x_data.iter().map(|x| x.powi(2)).sum::<f64>();

        let denominator = n * sum_x_squared - sum_x.powi(2);
        if denominator == 0.0 {
            return Err(
                "Denominator in slope calculation is zero, cannot compute slope and intercept",
            );
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        let intercept = (sum_y - slope * sum_x) / n;

        Ok((slope, intercept))
    }

    pub fn perform_linear_fit(&mut self) -> Result<(), &'static str> {
        // Ensure there's data to perform linear regression on
        if self.x_data.is_empty() || self.y_data.is_empty() {
            return Err("Insufficient data for linear regression.");
        }

        match Self::simple_linear_regression(&self.x_data, &self.y_data) {
            Ok((slope, intercept)) => {
                self.fit_params = Some(Parameters { slope, intercept });
                self.get_fit_line();
                log::info!(
                    "Background Fit (linear): slope: {}, intercept: {}",
                    slope,
                    intercept
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_fit_line(&mut self) {
        if let Some(params) = &self.fit_params {
            let x_min = self.x_data.iter().cloned().fold(f64::INFINITY, f64::min);
            let x_max = self.x_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

            let y_min = params.slope * x_min + params.intercept;
            let y_max = params.slope * x_max + params.intercept;

            self.fit_line = Some(vec![(x_min, y_min), (x_max, y_max)]);
        }
    }

    pub fn draw(&self, plot_ui: &mut egui_plot::PlotUi, color: egui::Color32) {
        if let Some(fit_line) = &self.fit_line {

            let plot_points: Vec<egui_plot::PlotPoint> = fit_line.iter().map(|(x, y)| egui_plot::PlotPoint::new(*x, *y)).collect();
            
            let line = egui_plot::Line::new(egui_plot::PlotPoints::Owned(plot_points))
                .color(color)
                .stroke(egui::Stroke::new(1.0, color));

            plot_ui.line(line);
        }
    }

}