use std::collections::BTreeMap;

use crate::FitError;

/// One side of a parameter bound.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Bound {
    /// No finite bound is present.
    Unbounded,
    /// The parameter may equal or remain beyond this finite boundary.
    Inclusive(f64),
}

impl Bound {
    pub(crate) fn lower_value(self) -> f64 {
        match self {
            Self::Unbounded => f64::NEG_INFINITY,
            Self::Inclusive(value) => value,
        }
    }

    pub(crate) fn upper_value(self) -> f64 {
        match self {
            Self::Unbounded => f64::INFINITY,
            Self::Inclusive(value) => value,
        }
    }
}

/// Inclusive lower and upper parameter bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Bounds {
    /// Lower parameter bound.
    pub lower: Bound,
    /// Upper parameter bound.
    pub upper: Bound,
}

impl Bounds {
    /// Creates an unbounded interval.
    #[must_use]
    pub const fn unbounded() -> Self {
        Self {
            lower: Bound::Unbounded,
            upper: Bound::Unbounded,
        }
    }

    /// Creates a finite inclusive interval.
    #[must_use]
    pub const fn finite(lower: f64, upper: f64) -> Self {
        Self {
            lower: Bound::Inclusive(lower),
            upper: Bound::Inclusive(upper),
        }
    }

    /// Creates an interval with a finite lower bound.
    #[must_use]
    pub const fn lower_bounded(lower: f64) -> Self {
        Self {
            lower: Bound::Inclusive(lower),
            upper: Bound::Unbounded,
        }
    }

    pub(crate) fn validate(self, initial: f64, name: &str) -> Result<(), FitError> {
        let lower = self.lower.lower_value();
        let upper = self.upper.upper_value();
        if lower.is_nan()
            || upper.is_nan()
            || !initial.is_finite()
            || lower >= upper
            || initial < lower
            || initial > upper
        {
            return Err(FitError::InvalidBounds {
                parameter: name.to_owned(),
            });
        }
        Ok(())
    }

    pub(crate) fn active(self, value: f64) -> bool {
        let scale = value.abs().max(1.0);
        let tolerance = 16.0 * f64::EPSILON * scale;
        (self.lower.lower_value().is_finite()
            && (value - self.lower.lower_value()).abs() <= tolerance)
            || (self.upper.upper_value().is_finite()
                && (value - self.upper.upper_value()).abs() <= tolerance)
    }
}

impl Default for Bounds {
    fn default() -> Self {
        Self::unbounded()
    }
}

/// A relationship that determines a parameter from another parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ParameterBinding {
    /// Keep this parameter equal to the named source parameter.
    EqualTo(String),
}

/// Definition of a model parameter before fitting.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ParameterDefinition {
    /// Globally unique parameter name.
    pub name: String,
    /// Initial external value.
    pub initial: f64,
    /// Inclusive bounds.
    pub bounds: Bounds,
    /// Whether the solver may vary this parameter.
    pub vary: bool,
    /// Optional shared-parameter binding.
    pub binding: Option<ParameterBinding>,
}

impl ParameterDefinition {
    /// Creates a varying, unbounded parameter.
    #[must_use]
    pub fn varying(name: impl Into<String>, initial: f64) -> Self {
        Self {
            name: name.into(),
            initial,
            bounds: Bounds::unbounded(),
            vary: true,
            binding: None,
        }
    }

    /// Creates a fixed, unbounded parameter.
    #[must_use]
    pub fn fixed(name: impl Into<String>, value: f64) -> Self {
        Self {
            name: name.into(),
            initial: value,
            bounds: Bounds::unbounded(),
            vary: false,
            binding: None,
        }
    }

    /// Applies bounds and returns the updated definition.
    #[must_use]
    pub const fn with_bounds(mut self, bounds: Bounds) -> Self {
        self.bounds = bounds;
        self
    }

    /// Applies an equal-to binding and returns the updated definition.
    #[must_use]
    pub fn equal_to(mut self, source: impl Into<String>) -> Self {
        self.vary = false;
        self.binding = Some(ParameterBinding::EqualTo(source.into()));
        self
    }
}

/// Classification of a fitted parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ParameterKind {
    /// Directly varied by the solver.
    Free,
    /// Held at its supplied value.
    Fixed,
    /// Bound to another parameter.
    Shared,
    /// Computed from fitted parameters.
    Derived,
}

/// A fitted or derived parameter and its propagated standard error.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ParameterEstimate {
    /// Parameter name.
    pub name: String,
    /// Best-fit value.
    pub value: f64,
    /// One-standard-deviation error, unavailable for singular covariance.
    pub standard_error: Option<f64>,
    /// Parameter classification.
    pub kind: ParameterKind,
    /// Bounds used by the fit.
    pub bounds: Bounds,
    /// Whether the estimate lies on a bound within numerical precision.
    pub active_bound: bool,
}

/// Ordered model parameter values keyed by name.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ParameterValues(BTreeMap<String, f64>);

impl ParameterValues {
    /// Creates an empty parameter map.
    #[must_use]
    pub const fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// Inserts a parameter value.
    pub fn insert(&mut self, name: impl Into<String>, value: f64) -> Option<f64> {
        self.0.insert(name.into(), value)
    }

    /// Returns a parameter value.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<f64> {
        self.0.get(name).copied()
    }

    /// Returns a parameter value or a typed missing-parameter error.
    pub fn require(&self, name: &str) -> Result<f64, FitError> {
        self.get(name).ok_or_else(|| FitError::InvalidParameter {
            parameter: name.to_owned(),
        })
    }

    /// Iterates through parameter names and values in deterministic name order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f64)> {
        self.0.iter().map(|(name, value)| (name.as_str(), *value))
    }
}

/// A value computed from fitted parameters with a gradient for error propagation.
#[derive(Debug, Clone, PartialEq)]
pub struct DerivedParameter {
    /// Derived parameter name.
    pub name: String,
    /// Derived value.
    pub value: f64,
    /// Derivatives with respect to named base parameters.
    pub gradient: Vec<(String, f64)>,
}
