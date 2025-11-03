#[derive(Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum IntegrationRule {
    Left,
    Right,
    Midpoint,
    Trapezoidal,
}

impl std::fmt::Display for IntegrationRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Left => "Left endpoint",
            Self::Right => "Right endpoint",
            Self::Midpoint => "Midpoint",
            Self::Trapezoidal => "Trapezoidal",
        };
        write!(f, "{s}")
    }
}

/// Left-endpoint rule on a non-uniform grid.
/// `x.len() == y.len()`, requires `x` strictly increasing.
#[inline]
pub fn integrate_left_endpoint(x: &[f64], y: &[f64]) -> f64 {
    debug_assert!(
        x.len() == y.len() && x.len() >= 2,
        "x and y must have the same length and at least 2 elements"
    );
    let mut s = 0.0;
    for i in 0..(x.len() - 1) {
        let dx = x[i + 1] - x[i];
        s += y[i] * dx;
    }
    s
}

/// Right-endpoint rule on a non-uniform grid.
#[inline]
pub fn integrate_right_endpoint(x: &[f64], y: &[f64]) -> f64 {
    debug_assert!(
        x.len() == y.len() && x.len() >= 2,
        "x and y must have the same length and at least 2 elements"
    );
    let mut s = 0.0;
    for i in 0..(x.len() - 1) {
        let dx = x[i + 1] - x[i];
        s += y[i + 1] * dx;
    }
    s
}

/// Midpoint rule on a non-uniform grid (linear interp within each cell).
/// Equivalent to using the average of the endpoints if y is linear per cell.
#[inline]
pub fn integrate_midpoint(x: &[f64], y: &[f64]) -> f64 {
    debug_assert!(
        x.len() == y.len() && x.len() >= 2,
        "x and y must have the same length and at least 2 elements"
    );
    let mut s = 0.0;
    for i in 0..(x.len() - 1) {
        let dx = x[i + 1] - x[i];
        let y_mid = 0.5 * (y[i] + y[i + 1]); // linear interp at midpoint
        s += y_mid * dx;
    }
    s
}

/// Trapezoidal rule on a non-uniform grid.
#[inline]
pub fn integrate_trapezoidal(x: &[f64], y: &[f64]) -> f64 {
    debug_assert!(
        x.len() == y.len() && x.len() >= 2,
        "x and y must have the same length and at least 2 elements"
    );
    trapz(y, Some(x), 1.0) // dx ignored when x is provided, matching NumPy
}

#[inline]
pub fn trapz(y: &[f64], x: Option<&[f64]>, dx: f64) -> f64 {
    let n = y.len();
    if n < 2 {
        return 0.0;
    }
    if let Some(xv) = x {
        debug_assert_eq!(xv.len(), n, "x and y must have same length");
        let mut s = 0.0;
        for i in 0..(n - 1) {
            // NOTE: no sorting; uses given sequence, can be non-monotonic
            s += 0.5 * (y[i] + y[i + 1]) * (xv[i + 1] - xv[i]);
        }
        s
    } else {
        // Evenly spaced grid with spacing dx
        let mut s = 0.0;
        for i in 0..(n - 1) {
            s += 0.5 * (y[i] + y[i + 1]) * dx;
        }
        s
    }
}

#[inline]
pub fn cell_polygon_with_baseline(
    rule: IntegrationRule,
    x0: f64,
    x1: f64,
    y0_vis: f64,
    y1_vis: f64,
    y_base: f64,
    log_y: bool,
) -> [[f64; 2]; 4] {
    // Convert values for log scale if enabled
    let (y0v, y1v, yb) = if log_y {
        (
            y0_vis.max(f64::MIN_POSITIVE).log10(),
            y1_vis.max(f64::MIN_POSITIVE).log10(),
            y_base.max(f64::MIN_POSITIVE).log10(),
        )
    } else {
        (y0_vis, y1_vis, y_base)
    };

    match rule {
        // rectangle at y = y0 (left)
        IntegrationRule::Left => [[x0, yb], [x0, y0v], [x1, y0v], [x1, yb]],
        // rectangle at y = y1 (right)
        IntegrationRule::Right => [[x0, yb], [x0, y1v], [x1, y1v], [x1, yb]],
        // rectangle at y = (y0+y1)/2 (midpoint)
        IntegrationRule::Midpoint => {
            let ym = 0.5 * (y0v + y1v);
            [[x0, yb], [x0, ym], [x1, ym], [x1, yb]]
        }
        // trapezoid connecting (x0,y0) → (x1,y1)
        IntegrationRule::Trapezoidal => [[x0, yb], [x0, y0v], [x1, y1v], [x1, yb]],
    }
}
