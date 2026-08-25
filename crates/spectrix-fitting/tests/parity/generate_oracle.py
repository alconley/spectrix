"""Regenerate the deterministic lmfit 1.3.4 compatibility fixture.

Run with the repository parity environment:
    .venv/Scripts/python crates/spectrix-fitting/tests/parity/generate_oracle.py \
        crates/spectrix-fitting/tests/parity/lmfit134_gaussian_linear.json
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

import lmfit
import numpy as np
import scipy


EXPECTED = ("1.3.4", "2.5.2", "1.18.1")


def synthetic_data() -> tuple[np.ndarray, np.ndarray]:
    x = np.linspace(0.0, 10.0, 101)
    gaussian = 120.0 / (math.sqrt(2.0 * math.pi) * 0.37) * np.exp(
        -0.5 * ((x - 5.1) / 0.37) ** 2
    )
    y = 0.7 * x + 2.0 + gaussian + 0.15 * np.sin(1.7 * x) + 0.03 * np.cos(4.2 * x)
    return x, y


def generate() -> dict[str, object]:
    versions = (lmfit.__version__, np.__version__, scipy.__version__)
    if versions != EXPECTED:
        raise RuntimeError(f"parity environment is {versions}, expected {EXPECTED}")

    centers, counts = synthetic_data()
    region = (2.0, 8.0)
    mask = (centers >= region[0]) & (centers <= region[1])
    x = centers[mask]
    y = counts[mask]

    bg_mask = ((centers >= 2.0) & (centers <= 3.0)) | (
        (centers >= 7.0) & (centers <= 8.0)
    )
    background = lmfit.models.LinearModel(prefix="bg_")
    background_parameters = background.make_params(slope=0.0, intercept=0.0)
    background_result = background.fit(counts[bg_mask], background_parameters, x=centers[bg_mask])
    parameters = background_result.params.copy()
    for parameter in parameters.values():
        parameter.set(vary=False)

    marker = 5.0
    peak_index = int(np.abs(x - marker).argmin())
    peak_height = y[peak_index]
    half_maximum = peak_height / 2.0
    left = np.where(y[:peak_index] <= half_maximum)[0]
    right = np.where(y[peak_index:] <= half_maximum)[0] + peak_index
    sigma = max((x[right[0]] - x[left[-1]]) / 2.3548, 0.2)
    adjusted_height = y[peak_index] - background_result.eval(x=marker)
    amplitude = max(float(adjusted_height * sigma / 0.3989423), 0.0)

    gaussian = lmfit.models.GaussianModel(prefix="g0_")
    model = background + gaussian
    parameters.update(gaussian.make_params(amplitude=amplitude, center=marker, sigma=sigma))
    parameters["g0_amplitude"].set(min=0.0)
    parameters["g0_sigma"].set(min=0.0)
    parameters["g0_center"].set(min=region[0], max=region[1])
    parameters.add("g0_area", expr="g0_amplitude / 0.1", min=0.0)

    result = model.fit(y, parameters, x=x)
    uncertainty = result.eval_uncertainty(x=x, sigma=1)
    covariance = None if result.covar is None else result.covar.tolist()
    correlations = None
    if result.covar is not None:
        diagonal = np.sqrt(np.maximum(np.diag(result.covar), 0.0))
        correlations = (
            result.covar / np.outer(diagonal, diagonal)
        ).tolist()

    sample_indices = list(range(0, len(x), 5))
    best_fit = result.eval(x=x)
    residuals = result.residual
    return {
        "versions": {"lmfit": versions[0], "numpy": versions[1], "scipy": versions[2]},
        "region": list(region),
        "peak_markers": [marker],
        "background_markers": [[2.0, 3.0], [7.0, 8.0]],
        "parameter_values": {name: float(parameter.value) for name, parameter in result.params.items()},
        "parameter_errors": {
            name: None if parameter.stderr is None else float(parameter.stderr)
            for name, parameter in result.params.items()
        },
        "variable_names": list(result.var_names),
        "covariance": covariance,
        "correlations": correlations,
        "sample_indices": sample_indices,
        "best_fit_samples": [float(best_fit[index]) for index in sample_indices],
        "residual_samples": [float(residuals[index]) for index in sample_indices],
        "uncertainty_samples": [float(uncertainty[index]) for index in sample_indices],
        "statistics": {
            "chi_square": float(result.chisqr),
            "reduced_chi_square": float(result.redchi),
            "aic": float(result.aic),
            "bic": float(result.bic),
            "r_squared": float(result.rsquared),
        },
    }


if __name__ == "__main__":
    encoded = json.dumps(generate(), indent=2, sort_keys=True) + "\n"
    if len(sys.argv) == 2:
        Path(sys.argv[1]).write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
