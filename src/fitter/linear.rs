use egui::Color32;
use egui_plot::{Line, PlotPoint, PlotPoints, PlotUi};
use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Parameters {
    pub slope: f64,
    pub intercept: f64,
}

impl Parameters {
    pub fn params_ui(&self, ui: &mut egui::Ui) {
        // just display the value with 4 decimal places
        ui.label(format!("Slope: {:.4}", self.slope));
        ui.label(format!("Intercept: {:.4}", self.intercept));
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinearFitter {
    x_data: Vec<f64>,
    y_data: Vec<f64>,
    pub fit_params: Option<Parameters>,
    fit_line: Option<Vec<(f64, f64)>>,
}

impl LinearFitter {
    /// Creates a new LinearFitter with the given data.
    pub fn new(x_data: Vec<f64>, y_data: Vec<f64>) -> Self {
        LinearFitter {
            x_data,
            y_data,
            fit_params: None,
            fit_line: None,
        }
    }

    /// Performs simple linear regression and returns the slope and intercept.
    pub fn simple_linear_regression(x_data: &[f64], y_data: &[f64]) -> (f64, f64) {
        assert!(
            x_data.len() == y_data.len() && !x_data.is_empty(),
            "x_data and y_data must have the same non-zero length"
        );

        let n = x_data.len() as f64;
        let sum_x: f64 = x_data.iter().sum();
        let sum_y: f64 = y_data.iter().sum();
        let sum_xy: f64 = x_data.iter().zip(y_data.iter()).map(|(x, y)| x * y).sum();
        let sum_x_squared: f64 = x_data.iter().map(|x| x.powi(2)).sum();

        let denominator = n * sum_x_squared - sum_x.powi(2);
        assert!(
            denominator != 0.0,
            "Denominator in slope calculation is zero, cannot compute slope and intercept"
        );

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        let intercept = (sum_y - slope * sum_x) / n;

        (slope, intercept)
    }

    // pub fn linear_regression(data_points: Vec<(f64, f64)>) -> Option<(f64, f64)> {
    //     if data_points.is_empty() {
    //         return None;
    //     }

    //     let count = data_points.len() as f64;
    //     let mean_x = data_points.iter().fold(0.0, |sum, y| sum + y.0) / count;
    //     let mean_y = data_points.iter().fold(0.0, |sum, y| sum + y.1) / count;

    //     let mut covariance = 0.0;
    //     let mut std_dev_sqr_x = 0.0;
    //     let mut std_dev_sqr_y = 0.0;

    //     for data_point in data_points {
    //         covariance += (data_point.0 - mean_x) * (data_point.1 - mean_y);
    //         std_dev_sqr_x += (data_point.0 - mean_x).powi(2);
    //         std_dev_sqr_y += (data_point.1 - mean_y).powi(2);
    //     }

    //     let std_dev_x = std_dev_sqr_x.sqrt();
    //     let std_dev_y = std_dev_sqr_y.sqrt();
    //     let std_dev_prod = std_dev_x * std_dev_y;

    //     let pcc = covariance / std_dev_prod; //Pearson's correlation constant
    //     let b = pcc * (std_dev_y / std_dev_x); //Slope of the line
    //     let a = mean_y - b * mean_x; //Y-Intercept of the line

    //     Some((a, b))
    // }

    /// Performs a linear fit on the provided data.
    pub fn perform_linear_fit(&mut self) {
        assert!(
            !self.x_data.is_empty() && !self.y_data.is_empty(),
            "Insufficient data for linear regression."
        );

        let (slope, intercept) = Self::simple_linear_regression(&self.x_data, &self.y_data);
        self.fit_params = Some(Parameters { slope, intercept });
        self.compute_fit_line();
        info!(
            "Background Fit (linear): slope: {}, intercept: {}",
            slope, intercept
        );
    }

    /// Computes the fit line based on the fit parameters.
    fn compute_fit_line(&mut self) {
        if let Some(params) = &self.fit_params {
            let (x_min, x_max) = self
                .x_data
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &x| {
                    (min.min(x), max.max(x))
                });

            let y_min = params.slope * x_min + params.intercept;
            let y_max = params.slope * x_max + params.intercept;

            self.fit_line = Some(vec![(x_min, y_min), (x_max, y_max)]);
        }
    }

    /// Draws the fit line on the given plot UI.
    pub fn draw(&self, plot_ui: &mut PlotUi, color: Color32) {
        if let Some(fit_line) = &self.fit_line {
            let plot_points: Vec<PlotPoint> = fit_line
                .iter()
                .map(|(x, y)| PlotPoint::new(*x, *y))
                .collect();

            let line = Line::new(PlotPoints::Owned(plot_points))
                .color(color)
                .stroke(egui::Stroke::new(1.0, color));

            plot_ui.line(line);
        }
    }

    /// Calculates the background values based on the fit parameters.
    pub fn calculate_background(&self, x_data: &[f64]) -> Vec<f64> {
        if let Some(params) = &self.fit_params {
            x_data
                .iter()
                .map(|&x| params.slope * x + params.intercept)
                .collect()
        } else {
            vec![]
        }
    }

    pub fn fit_params_ui(&self, ui: &mut egui::Ui) {
        if let Some(params) = &self.fit_params {
            params.params_ui(ui);
        }
    }
}

// Unit tests for LinearFitter
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_linear_regression() {
        let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y_data = vec![2.0, 4.0, 5.0, 4.0, 5.0];
        let (slope, intercept) = LinearFitter::simple_linear_regression(&x_data, &y_data);
        assert!((slope - 0.6).abs() < 1e-6);
        assert!((intercept - 2.2).abs() < 1e-6);
    }

    #[test]
    fn test_perform_linear_fit() {
        let x_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y_data = vec![2.0, 4.0, 5.0, 4.0, 5.0];
        let mut fitter = LinearFitter::new(x_data, y_data);
        fitter.perform_linear_fit();
        let params = fitter.fit_params.unwrap();
        assert!((params.slope - 0.6).abs() < 1e-6);
        assert!((params.intercept - 2.2).abs() < 1e-6);
    }
}
