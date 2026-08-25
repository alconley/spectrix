"""Generate deterministic lmfit 1.3.4 fixtures for every v1 background."""

from __future__ import annotations

import json
import sys
from pathlib import Path

import lmfit
import numpy as np
import scipy


EXPECTED = ("1.3.4", "2.5.2", "1.18.1")


def cases():
    x = np.linspace(1.0, 8.0, 141)
    noise = 0.007 * np.sin(1.31 * x) + 0.003 * np.cos(3.7 * x)
    return [
        ("constant", lmfit.models.ConstantModel(), {"c": 0.0}, 4.2 + noise),
        (
            "linear",
            lmfit.models.LinearModel(),
            {"slope": 0.0, "intercept": 0.0},
            0.7 * x + 2.1 + noise,
        ),
        (
            "quadratic",
            lmfit.models.QuadraticModel(),
            {"a": 0.0, "b": 0.0, "c": 0.0},
            0.05 * x**2 - 0.4 * x + 3.2 + noise,
        ),
        (
            "exponential",
            lmfit.models.ExponentialModel(),
            {"amplitude": 5.0, "decay": 2.0},
            8.0 * np.exp(-x / 3.0) + noise,
        ),
        (
            "power_law",
            lmfit.models.PowerLawModel(),
            {"amplitude": 5.0, "exponent": -1.0},
            6.0 * x**-0.8 + noise,
        ),
    ], x


def generate():
    versions = (lmfit.__version__, np.__version__, scipy.__version__)
    if versions != EXPECTED:
        raise RuntimeError(f"parity environment is {versions}, expected {EXPECTED}")
    definitions, x = cases()
    sample_indices = list(range(0, len(x), 10))
    output = []
    for name, model, initial, y in definitions:
        result = model.fit(y, model.make_params(**initial), x=x)
        uncertainty = result.eval_uncertainty(x=x, sigma=1)
        diagonal = np.sqrt(np.maximum(np.diag(result.covar), 0.0))
        output.append(
            {
                "name": name,
                "initial": initial,
                "x": x.tolist(),
                "y": y.tolist(),
                "values": {key: float(value.value) for key, value in result.params.items()},
                "errors": {key: float(value.stderr) for key, value in result.params.items()},
                "variable_names": result.var_names,
                "covariance": result.covar.tolist(),
                "correlations": (result.covar / np.outer(diagonal, diagonal)).tolist(),
                "sample_indices": sample_indices,
                "best_fit_samples": [float(result.best_fit[index]) for index in sample_indices],
                "residual_samples": [float(result.residual[index]) for index in sample_indices],
                "uncertainty_samples": [float(uncertainty[index]) for index in sample_indices],
                "statistics": {
                    "chi_square": float(result.chisqr),
                    "reduced_chi_square": float(result.redchi),
                    "aic": float(result.aic),
                    "bic": float(result.bic),
                    "r_squared": float(result.rsquared),
                },
            }
        )
    return {"versions": versions, "cases": output}


if __name__ == "__main__":
    encoded = json.dumps(generate(), indent=2, sort_keys=True) + "\n"
    if len(sys.argv) == 2:
        Path(sys.argv[1]).write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
