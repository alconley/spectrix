use core::fmt;

/// Errors reported while validating or evaluating a fit.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum FitError {
    /// Independent and dependent data have different lengths.
    LengthMismatch {
        /// Length of the independent or expected collection.
        x: usize,
        /// Length of the dependent or actual collection.
        y: usize,
    },
    /// A required input collection is empty.
    EmptyData,
    /// A region is missing, degenerate, or contains no samples.
    InvalidRegion,
    /// A parameter has invalid bounds or an initial value outside its bounds.
    InvalidBounds {
        /// Name of the invalid parameter.
        parameter: String,
    },
    /// A parameter name is missing or duplicated.
    InvalidParameter {
        /// Name of the invalid parameter.
        parameter: String,
    },
    /// A model is undefined for the supplied independent variable or parameters.
    Domain {
        /// Model reporting the domain failure.
        model: String,
        /// Description of the required domain.
        message: String,
    },
    /// An input or model evaluation is not finite.
    NonFinite {
        /// Input or calculation containing the non-finite value.
        context: String,
    },
    /// There are not enough observations for the number of free parameters.
    InsufficientDegreesOfFreedom {
        /// Number of observations.
        observations: usize,
        /// Number of independently varying parameters.
        variables: usize,
    },
    /// The requested allocation exceeds the crate's configured safety limit.
    AllocationLimit {
        /// Number of scalar values requested.
        requested: usize,
        /// Maximum number of scalar values permitted.
        limit: usize,
    },
    /// The optimizer could not produce usable parameter estimates.
    Solver {
        /// Solver configuration or failure detail.
        message: String,
    },
}

impl fmt::Display for FitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LengthMismatch { x, y } => write!(formatter, "x/y length mismatch ({x} != {y})"),
            Self::EmptyData => formatter.write_str("fit data is empty"),
            Self::InvalidRegion => formatter.write_str("fit region is invalid or contains no data"),
            Self::InvalidBounds { parameter } => {
                write!(formatter, "invalid bounds for parameter `{parameter}`")
            }
            Self::InvalidParameter { parameter } => {
                write!(formatter, "invalid or duplicate parameter `{parameter}`")
            }
            Self::Domain { model, message } => write!(formatter, "{model} domain error: {message}"),
            Self::NonFinite { context } => write!(formatter, "non-finite value in {context}"),
            Self::InsufficientDegreesOfFreedom {
                observations,
                variables,
            } => write!(
                formatter,
                "insufficient degrees of freedom: {observations} observations, {variables} variables"
            ),
            Self::AllocationLimit { requested, limit } => write!(
                formatter,
                "requested allocation ({requested}) exceeds safety limit ({limit})"
            ),
            Self::Solver { message } => write!(formatter, "solver error: {message}"),
        }
    }
}

impl std::error::Error for FitError {}
