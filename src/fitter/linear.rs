use log::info;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinearParameters {
    pub slope: f64,
    pub intercept: f64,
}

impl LinearParameters {
    pub fn params_ui(&self, ui: &mut egui::Ui) {
        // just display the value with 4 decimal places
        ui.label(format!("Slope: {:.4}", self.slope));
        ui.label(format!("Intercept: {:.4}", self.intercept));
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LinearFitter {
    pub x_data: Vec<f64>,
    pub y_data: Vec<f64>,
    pub fit_params: Option<LinearParameters>,
    pub fit_points: Option<Vec<[f64; 2]>>,
}

impl LinearFitter {
    /// Creates a new LinearFitter with the given data.
    pub fn new(x_data: Vec<f64>, y_data: Vec<f64>) -> Self {
        LinearFitter {
            x_data,
            y_data,
            fit_params: None,
            fit_points: None,
        }
    }

    pub fn linear_regression(data_points: Vec<(f64, f64)>) -> Option<(f64, f64)> {
        if data_points.is_empty() {
            return None;
        }

        let count = data_points.len() as f64;
        let mean_x = data_points.iter().map(|(x, _)| x).sum::<f64>() / count;
        let mean_y = data_points.iter().map(|(_, y)| y).sum::<f64>() / count;

        let mut covariance = 0.0;
        let mut std_dev_sqr_x = 0.0;
        let mut std_dev_sqr_y = 0.0;

        for (x, y) in data_points {
            covariance += (x - mean_x) * (y - mean_y);
            std_dev_sqr_x += (x - mean_x).powi(2);
            std_dev_sqr_y += (y - mean_y).powi(2);
        }

        let std_dev_x = std_dev_sqr_x.sqrt();
        let std_dev_y = std_dev_sqr_y.sqrt();
        let std_dev_prod = std_dev_x * std_dev_y;

        let pcc = covariance / std_dev_prod; // Pearson's correlation constant
        let slope = pcc * (std_dev_y / std_dev_x); // Slope of the line
        let intercept = mean_y - slope * mean_x; // Y-Intercept of the line

        Some((slope, intercept))
    }

    /// Performs a linear fit on the provided data.
    pub fn perform_linear_fit(&mut self) {
        assert!(
            !self.x_data.is_empty() && !self.y_data.is_empty(),
            "Insufficient data for linear regression."
        );

        let data_points: Vec<(f64, f64)> = self
            .x_data
            .iter()
            .zip(&self.y_data)
            .map(|(&x, &y)| (x, y))
            .collect();
        if let Some((slope, intercept)) = Self::linear_regression(data_points) {
            self.fit_params = Some(LinearParameters { slope, intercept });
            self.compute_fit_line();
            info!(
                "Background Fit (linear): slope: {}, intercept: {}",
                slope, intercept
            );
        } else {
            self.fit_params = None;
        }
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

            let points = vec![[x_min, y_min], [x_max, y_max]];

            self.fit_points = Some(points);
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
    fn test_linear_regression() {
        let data_points = vec![(1.0, 2.0), (2.0, 4.0), (3.0, 5.0), (4.0, 4.0), (5.0, 5.0)];
        let (slope, intercept) = LinearFitter::linear_regression(data_points).unwrap();
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
