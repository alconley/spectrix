# spectrix-fitting

`spectrix-fitting` is a safe, deterministic Rust crate for nonlinear least-squares and Poisson-deviance fitting of spectra. It has no Python or UI dependency and does not expose its internal linear-algebra types. Version 0.1 provides:

- unit-normalized Gaussian peaks (`amplitude` is the integral), including `height`, `fwhm`, and Spectrix `area = amplitude / bin_width`;
- none/constant, linear, quadratic, exponential, and power-law backgrounds;
- fixed, bounded, shared, and derived parameters;
- likelihood covariance for Poisson fits and reduced-chi-square-scaled covariance for least squares;
- normal-based Poisson confidence bands and Student-t-scaled least-squares bands;
- a MINPACK-compatible `Lmfit134` solver profile; and
- spectrum preprocessing compatible with Spectrix marker workflows.

The crate forbids unsafe code and uses checked allocation limits. Singular covariance is returned as unavailable; it is never synthesized.

## Peak fitting

```rust
use spectrix_fitting::{
    fit_peaks, BackgroundCoupling, BackgroundKind, FitOptions, ManualPeakSeed,
    ObjectiveKind, PeakFitRequest,
};

let x = (0..101).map(|i| i as f64 * 0.1).collect::<Vec<_>>();
let y = x.iter().map(|x| {
    2.0 + 80.0 / ((2.0 * std::f64::consts::PI).sqrt() * 0.3)
        * (-0.5 * ((*x - 5.0) / 0.3).powi(2)).exp()
}).collect::<Vec<_>>();

let request = PeakFitRequest {
    x,
    y,
    bin_width: 0.1,
    region: [2.0, 8.0],
    peak_seeds: vec![ManualPeakSeed { center: 5.0, sigma: 0.3, amplitude: 80.0 }],
    peak_bounds: None,
    background_markers: vec![(2.0, 3.0), (7.0, 8.0)],
    background: BackgroundKind::Constant,
    background_seed: None,
    background_coupling: BackgroundCoupling::PrefitJoint,
    equal_sigma: true,
    free_centers: true,
    sigma_bounds: None,
};

let mut options = FitOptions::default();
options.objective = ObjectiveKind::PoissonDeviance;
let result = fit_peaks(&request, &options)?;
assert!(result.fit.termination.success);
assert!(result.fit.covariance.is_some());
assert!(result.fit.confidence_band.is_some());
# Ok::<(), spectrix_fitting::FitError>(())
```

Use `BackgroundCoupling::PrefitFrozen` only with explicit fixed background seed values (Spectrix's **Lock manual background** workflow). Use `BackgroundCoupling::PrefitJoint` to initialize the selected background in the explicit marker windows and vary it with the peaks, including cross-correlation. Peak seeds are required; non-`None` backgrounds require at least one explicit marker window. Optional `ManualPeakBounds` constrain position, sigma, and net bin height one-for-one with the seeds. The fitter varies net height directly so width changes do not implicitly move the peak height, then reports integrated amplitude and area as derived parameters. It performs one deterministic composite solve from the supplied seeds.

Histogram Gaussians are integrated across bin edges while retaining Spectrix's existing amplitude and area conventions.

## Custom and composite models

Implement the object-safe `Model` trait for a custom equation. Parameters use names rather than public nalgebra types, and an analytic Jacobian is optional:

```rust
use spectrix_fitting::{
    FitError, Model, ParameterDefinition, ParameterValues,
};

struct Offset;

impl Model for Offset {
    fn name(&self) -> &str { "offset" }

    fn parameter_definitions(&self) -> Vec<ParameterDefinition> {
        vec![ParameterDefinition::varying("offset", 0.0)]
    }

    fn evaluate(
        &self,
        x: &[f64],
        parameters: &ParameterValues,
        output: &mut [f64],
    ) -> Result<(), FitError> {
        if x.len() != output.len() {
            return Err(FitError::LengthMismatch { x: x.len(), y: output.len() });
        }
        output.fill(parameters.require("offset")?);
        Ok(())
    }
}
```

Built-ins can be combined with `CompositeModel` and `ModelComponent`. Prefix each component's parameter names (for example `g0_` and `g1_`) so the namespace remains unique. `ParameterDefinition::equal_to` creates shared bindings such as equal sigma.

## Compatibility testing

The committed oracles in `tests/parity` are generated with lmfit 1.3.4, NumPy 2.5.2, and SciPy 1.18.1. They cover every V1 background equation and a high-level matrix of one, three, and overlapping peaks; equal and independent sigma; fixed and free centers; sigma constraints; marker fallback/filtering; reversed regions; frozen and joint coupling; and bounded background seeds. Regenerate them only in that pinned environment, then run:

```text
cargo test -p spectrix-fitting --all-features
```

Compatibility thresholds are `rtol=1e-8, atol=1e-10` for fitted values, curves, residuals, and statistics, and `rtol=1e-6, atol=1e-9` for covariance, correlations, errors, and bands.

## Performance gate

Run `benches/compare.ps1` from the workspace root with the pinned parity environment. The warmed release-mode matrix covers 1, 3, and 8 fixed-center peaks over 512, 2048, and 8192 bins, including preprocessing, solve, covariance, component curves, and complete total/component band payloads. The committed [`PERFORMANCE.md`](PERFORMANCE.md) records the latest passing run.

## License

Licensed under either Apache-2.0 or MIT, at your option.
