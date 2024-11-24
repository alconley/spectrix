use super::cuts::Cut;

// Enum to encapsulate 1D and 2D histogram configurations
#[derive(Clone, serde::Deserialize, serde::Serialize, Debug)]
pub enum Configs {
    Hist1D(Hist1DConfig),
    Hist2D(Hist2DConfig),
}

impl Configs {
    /// Create a new 1D histogram configuration.
    pub fn new_1d(
        name: &str,
        column_name: &str,
        range: (f64, f64),
        bins: usize,
        cuts: Option<Vec<Cut>>,
    ) -> Self {
        let mut config = Hist1DConfig::new(name, column_name, range, bins);
        if let Some(cuts) = cuts {
            config.cuts = cuts;
        }
        Configs::Hist1D(config)
    }

    /// Create a new 2D histogram configuration.
    pub fn new_2d(
        name: &str,
        x_column_name: &str,
        y_column_name: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        bins: (usize, usize),
        cuts: Option<Vec<Cut>>,
    ) -> Self {
        let mut config =
            Hist2DConfig::new(name, x_column_name, y_column_name, x_range, y_range, bins);
        if let Some(cuts) = cuts {
            config.cuts = cuts;
        }
        Configs::Hist2D(config)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Hist1DConfig {
    pub name: String,        // Histogram display name
    pub column_name: String, // Data column to fill from
    pub range: (f64, f64),   // Range for the histogram
    pub bins: usize,         // Number of bins
    pub cuts: Vec<Cut>,      // Cuts for the histogram
    pub calculate: bool,     // Whether to calculate the histogram
}

impl Hist1DConfig {
    pub fn new(name: &str, column_name: &str, range: (f64, f64), bins: usize) -> Self {
        Self {
            name: name.to_string(),
            column_name: column_name.to_string(),
            range,
            bins,
            cuts: Vec::new(),
            calculate: true,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub struct Hist2DConfig {
    pub name: String,          // Histogram display name
    pub x_column_name: String, // Data column for X-axis
    pub y_column_name: String, // Data column for Y-axis
    pub x_range: (f64, f64),   // Range for X-axis
    pub y_range: (f64, f64),   // Range for Y-axis
    pub bins: (usize, usize),  // Number of bins for X and Y axes
    pub cuts: Vec<Cut>,        // Cuts for the histogram
    pub calculate: bool,       // Whether to calculate the histogram
}

impl Hist2DConfig {
    pub fn new(
        name: &str,
        x_column_name: &str,
        y_column_name: &str,
        x_range: (f64, f64),
        y_range: (f64, f64),
        bins: (usize, usize),
    ) -> Self {
        Self {
            name: name.to_string(),
            x_column_name: x_column_name.to_string(),
            y_column_name: y_column_name.to_string(),
            x_range,
            y_range,
            bins,
            cuts: Vec::new(),
            calculate: true,
        }
    }
}
