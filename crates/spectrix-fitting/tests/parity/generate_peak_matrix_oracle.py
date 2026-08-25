"""Generate the deterministic high-level Gaussian compatibility matrix.

Run with the repository parity environment::

    .venv/Scripts/python \
      crates/spectrix-fitting/tests/parity/generate_peak_matrix_oracle.py \
      crates/spectrix-fitting/tests/parity/lmfit134_peak_matrix.json
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from typing import Any

import lmfit
import numpy as np
import scipy


EXPECTED = ("1.3.4", "2.5.2", "1.18.1")
SQRT_TWO_PI = math.sqrt(2.0 * math.pi)


def gaussian(x: np.ndarray, amplitude: float, center: float, sigma: float) -> np.ndarray:
    return amplitude / (SQRT_TWO_PI * sigma) * np.exp(-0.5 * ((x - center) / sigma) ** 2)


def case_data(
    background: str,
    peaks: list[tuple[float, float, float]],
    *,
    noise: float = 0.12,
) -> tuple[np.ndarray, np.ndarray]:
    # Store the intended decimal bin centers, avoiding parser-dependent rounding
    # of the long representations produced by linspace at a few endpoints.
    x = np.round(np.linspace(0.1, 12.0, 120), 1)
    if background == "none":
        y = np.zeros_like(x)
    elif background == "constant":
        y = np.full_like(x, 3.2)
    elif background == "linear":
        y = 0.31 * x + 1.7
    elif background == "quadratic":
        y = 0.018 * x * x - 0.12 * x + 2.4
    elif background == "exponential":
        y = 5.5 * np.exp(-x / 6.0)
    elif background == "power_law":
        y = 4.0 * x**-0.42
    else:
        raise ValueError(background)
    for amplitude, center, sigma in peaks:
        y = y + gaussian(x, amplitude, center, sigma)
    y = y + noise * np.sin(1.31 * x) + 0.03 * np.cos(3.7 * x)
    return x, y


def cases() -> list[dict[str, Any]]:
    return [
        {
            "name": "single_none_fixed_center_reversed_region",
            "background": "none",
            "peaks": [(72.0, 5.0, 0.38)],
            "region": [8.0, 2.0],
            "peak_markers": [5.0],
            "background_markers": [],
            "coupling": "frozen",
            "equal_sigma": True,
            "free_centers": False,
            "sigma_bounds": None,
        },
        {
            "name": "two_equal_frozen_constant",
            "background": "constant",
            "peaks": [(52.0, 3.4, 0.34), (81.0, 7.2, 0.34)],
            "region": [1.5, 9.2],
            "peak_markers": [7.1, 3.5],
            "background_markers": [[2.6, 1.6], [9.1, 8.2]],
            "coupling": "frozen",
            "equal_sigma": True,
            "free_centers": True,
            "sigma_bounds": None,
        },
        {
            "name": "three_independent_frozen_linear_constrained",
            "background": "linear",
            "peaks": [(48.0, 2.8, 0.27), (75.0, 5.8, 0.43), (38.0, 8.7, 0.31)],
            "region": [1.2, 10.2],
            "peak_markers": [-4.0, 8.8, 2.7, 5.9, 20.0],
            "background_markers": [[1.2, 1.9], [9.5, 10.2]],
            "coupling": "frozen",
            "equal_sigma": False,
            "free_centers": True,
            "sigma_bounds": {"minima": [0.15, 0.2, 0.15], "maxima": [0.7, 0.8, 0.7]},
        },
        {
            "name": "overlapping_equal_joint_linear",
            "background": "linear",
            "peaks": [(66.0, 5.0, 0.39), (44.0, 5.8, 0.39)],
            "region": [3.0, 8.0],
            "peak_markers": [4.95, 5.85],
            "background_markers": [[3.0, 3.7], [7.3, 8.0]],
            "coupling": "joint",
            "equal_sigma": True,
            "free_centers": True,
            "sigma_bounds": None,
        },
        {
            "name": "fallback_frozen_quadratic",
            "background": "quadratic",
            "peaks": [(63.0, 6.2, 0.41)],
            "region": [3.0, 9.5],
            "peak_markers": [],
            "background_markers": [[3.0, 4.1], [8.5, 9.5]],
            "coupling": "frozen",
            "equal_sigma": True,
            "free_centers": False,
            "sigma_bounds": None,
        },
        {
            "name": "bounded_sigma_frozen_exponential",
            "background": "exponential",
            "peaks": [(58.0, 6.0, 0.46)],
            "region": [2.0, 10.0],
            "peak_markers": [6.0],
            "background_markers": [[2.0, 3.3], [8.7, 10.0]],
            "coupling": "frozen",
            "equal_sigma": True,
            "free_centers": False,
            "sigma_bounds": {"minima": [0.2], "maxima": [0.6]},
        },
        {
            "name": "single_frozen_power_law",
            "background": "power_law",
            "peaks": [(70.0, 4.7, 0.36)],
            "region": [1.0, 9.0],
            "peak_markers": [4.8],
            "background_markers": [[1.0, 2.0], [8.0, 9.0]],
            "coupling": "frozen",
            "equal_sigma": True,
            "free_centers": False,
            "sigma_bounds": None,
        },
        {
            "name": "seeded_bounded_frozen_linear",
            "background": "linear",
            "peaks": [(54.0, 6.7, 0.33)],
            "region": [3.0, 10.0],
            "peak_markers": [6.6],
            "background_markers": [[3.0, 4.0], [9.0, 10.0]],
            "coupling": "frozen",
            "equal_sigma": True,
            "free_centers": False,
            "sigma_bounds": None,
            "background_seed": [
                {"name": "bg_slope", "initial": 0.15, "minimum": -1.0, "maximum": 1.0, "vary": True},
                {"name": "bg_intercept", "initial": 1.5, "minimum": 0.0, "maximum": 4.0, "vary": False},
            ],
        },
    ]


def background_model(kind: str) -> lmfit.Model:
    if kind in ("none", "constant"):
        return lmfit.models.ConstantModel(prefix="bg_")
    if kind == "linear":
        return lmfit.models.LinearModel(prefix="bg_")
    if kind == "quadratic":
        return lmfit.models.QuadraticModel(prefix="bg_")
    if kind == "exponential":
        return lmfit.models.ExponentialModel(prefix="bg_")
    if kind == "power_law":
        return lmfit.models.PowerLawModel(prefix="bg_")
    raise ValueError(kind)


def background_params(model: lmfit.Model, kind: str, seed: list[dict[str, Any]] | None) -> lmfit.Parameters:
    defaults: dict[str, float]
    if kind in ("none", "constant"):
        defaults = {"c": 0.0}
    elif kind == "linear":
        defaults = {"slope": 0.0, "intercept": 0.0}
    elif kind == "quadratic":
        defaults = {"a": 0.0, "b": 0.0, "c": 0.0}
    elif kind == "exponential":
        defaults = {"amplitude": 0.0, "decay": 500.0}
    else:
        defaults = {"amplitude": 0.0, "exponent": -1.0}
    params = model.make_params(**defaults)
    if kind == "none":
        params["bg_c"].set(vary=False)
    for definition in seed or []:
        params[definition["name"]].set(
            value=definition["initial"],
            min=definition["minimum"],
            max=definition["maximum"],
            vary=definition["vary"],
        )
    return params


def selected_region(x: np.ndarray, y: np.ndarray, region: list[float]) -> tuple[np.ndarray, np.ndarray, list[float]]:
    ordered = sorted(region)
    mask = (x >= ordered[0]) & (x <= ordered[1])
    return x[mask], y[mask], ordered


def background_points(
    x: np.ndarray,
    y: np.ndarray,
    region: list[float],
    windows: list[list[float]],
) -> tuple[np.ndarray, np.ndarray]:
    use_windows = windows or [[region[0] - 0.1, region[0]], [region[1], region[1] + 0.1]]
    selected_x: list[float] = []
    selected_y: list[float] = []
    for window in use_windows:
        lower, upper = sorted(window)
        mask = (x >= lower) & (x <= upper)
        selected_x.extend(x[mask])
        selected_y.extend(y[mask])
    return np.asarray(selected_x), np.asarray(selected_y)


def nearest_index(x: np.ndarray, marker: float) -> int:
    return int(np.abs(x - marker).argmin())


def estimate_sigma(x: np.ndarray, y: np.ndarray, marker: float) -> float:
    peak = nearest_index(x, marker)
    half = y[peak] / 2.0
    left = np.flatnonzero(y[:peak] <= half)
    right = np.flatnonzero(y[peak:] <= half) + peak
    if left.size and right.size:
        return max(float((x[right[0]] - x[left[-1]]) / 2.3548), 0.2)
    return 0.2


def active(value: float, minimum: float, maximum: float) -> bool:
    tolerance = 16.0 * np.finfo(float).eps * max(abs(value), 1.0)
    return (math.isfinite(minimum) and abs(value - minimum) <= tolerance) or (
        math.isfinite(maximum) and abs(value - maximum) <= tolerance
    )


def serialize_result(
    result: lmfit.model.ModelResult,
    evaluation_x: np.ndarray,
    component_name_map: dict[str, str],
) -> dict[str, Any]:
    best_fit = np.asarray(result.eval(x=evaluation_x), dtype=float)
    components = {
        component_name_map.get(name, name): np.asarray(values, dtype=float).tolist()
        for name, values in result.eval_components(x=evaluation_x).items()
    }
    covariance = None if result.covar is None else result.covar.tolist()
    correlations = None
    uncertainty = None
    component_uncertainties = None
    if result.covar is not None and not any(parameter.stderr is None for parameter in result.params.values()):
        uncertainty_values = np.asarray(result.eval_uncertainty(x=evaluation_x, sigma=1), dtype=float)
        uncertainty = uncertainty_values.tolist()
        component_uncertainties = {
            component_name_map.get(name, name): np.asarray(values, dtype=float).tolist()
            for name, values in result.dely_comps.items()
        }
        diagonal = np.sqrt(np.maximum(np.diag(result.covar), 0.0))
        correlations = (result.covar / np.outer(diagonal, diagonal)).tolist()

    parameters = []
    for name, parameter in result.params.items():
        if name in result.var_names:
            kind = "free"
        elif parameter.expr is not None and parameter.expr in result.var_names:
            kind = "shared"
        elif parameter.expr is not None:
            kind = "derived"
        else:
            kind = "fixed"
        is_active = (
            False
            if kind == "derived"
            else bool(active(parameter.value, parameter.min, parameter.max))
        )
        parameters.append(
            {
                "name": name,
                "value": float(parameter.value),
                "standard_error": None if parameter.stderr is None else float(parameter.stderr),
                "kind": kind,
                "active_bound": is_active,
            }
        )
    residual_values = np.asarray(result.residual, dtype=float)
    residual_sample_indices = np.flatnonzero(np.abs(residual_values) >= 0.08).tolist()
    if not residual_sample_indices:
        residual_sample_indices = [int(np.argmax(np.abs(residual_values)))]
    return {
        "success": bool(result.success),
        "parameters": parameters,
        "statistics": {
            "chi_square": float(result.chisqr),
            "reduced_chi_square": float(result.redchi),
            "aic": float(result.aic),
            "bic": float(result.bic),
            "r_squared": float(result.rsquared),
        },
        "residual_sample_indices": residual_sample_indices,
        "residual_samples": [
            float(residual_values[index]) for index in residual_sample_indices
        ],
        "best_fit": best_fit.tolist(),
        "components": components,
        "variable_names": list(result.var_names),
        "covariance": covariance,
        "correlations": correlations,
        "uncertainty": uncertainty,
        "component_uncertainties": component_uncertainties,
    }


def fit_case(case: dict[str, Any]) -> dict[str, Any]:
    centers, counts = case_data(case["background"], case["peaks"])
    x, y, region = selected_region(centers, counts, case["region"])
    markers = sorted(
        marker for marker in case["peak_markers"] if region[0] <= marker <= region[1]
    )
    if not markers:
        markers = [float(x[int(np.argmax(y))])]

    bg_x, bg_y = background_points(
        centers, counts, region, case["background_markers"]
    )
    background = background_model(case["background"])
    seed = case.get("background_seed")
    prefit_params = background_params(background, case["background"], seed)
    background_result = background.fit(bg_y, prefit_params, x=bg_x)

    parameters = background_result.params.copy()
    seed_by_name = {definition["name"]: definition for definition in seed or []}
    for name, parameter in parameters.items():
        definition = seed_by_name.get(name)
        allowed_to_vary = parameter.vary if definition is None else definition["vary"]
        parameter.set(vary=allowed_to_vary and case["coupling"] == "joint")

    background_values = background_result.eval(x=np.asarray(markers))
    heights = [float(y[nearest_index(x, marker)]) for marker in markers]
    sigma_estimates = [estimate_sigma(x, y, marker) for marker in markers]
    strongest = int(np.argmax(heights))
    shared_sigma = sigma_estimates[strongest]

    model: lmfit.Model = background
    for index, marker in enumerate(markers):
        prefix = f"g{index}_"
        sigma = shared_sigma if case["equal_sigma"] else sigma_estimates[index]
        sigma_bounds = case["sigma_bounds"]
        if sigma_bounds is None:
            sigma_minimum, sigma_maximum = 0.0, np.inf
        else:
            constraint_index = 0 if case["equal_sigma"] else index
            sigma_minimum = sigma_bounds["minima"][constraint_index]
            sigma_maximum = sigma_bounds["maxima"][constraint_index]
        sigma = min(max(sigma, sigma_minimum), sigma_maximum)
        amplitude = max(float((heights[index] - background_values[index]) * shared_sigma / 0.3989423), 0.0)

        previous = region[0] if index == 0 else markers[index - 1]
        following = region[1] if index + 1 == len(markers) else markers[index + 1]
        center_minimum = max(previous if abs(marker - previous) <= shared_sigma else marker - shared_sigma, region[0])
        center_maximum = min(following if abs(following - marker) <= shared_sigma else marker + shared_sigma, region[1])

        peak = lmfit.models.GaussianModel(prefix=prefix)
        model = model + peak
        peak_parameters = peak.make_params(amplitude=amplitude, center=marker, sigma=sigma)
        peak_parameters[f"{prefix}amplitude"].set(min=0.0)
        peak_parameters[f"{prefix}center"].set(
            min=center_minimum,
            max=center_maximum,
            vary=case["free_centers"],
        )
        if case["equal_sigma"] and index > 0:
            peak_parameters[f"{prefix}sigma"].set(expr="g0_sigma")
        else:
            peak_parameters[f"{prefix}sigma"].set(min=sigma_minimum, max=sigma_maximum)
        peak_parameters.add(f"{prefix}area", expr=f"{prefix}amplitude / 0.1", min=0.0)
        parameters.update(peak_parameters)

    result = model.fit(y, parameters, x=x)
    encoded = dict(case)
    encoded.pop("peaks")
    encoded["x"] = centers.tolist()
    encoded["y"] = counts.tolist()
    encoded["used_region"] = region
    encoded["used_peak_markers"] = markers
    encoded["evaluation_x"] = x.tolist()
    encoded["fit"] = serialize_result(result, x, {"bg_": "background"})
    encoded["background_prefit"] = serialize_result(background_result, x, {})
    return encoded


def generate() -> dict[str, Any]:
    versions = (lmfit.__version__, np.__version__, scipy.__version__)
    if versions != EXPECTED:
        raise RuntimeError(f"parity environment is {versions}, expected {EXPECTED}")
    return {
        "versions": {"lmfit": versions[0], "numpy": versions[1], "scipy": versions[2]},
        "cases": [fit_case(case) for case in cases()],
    }


if __name__ == "__main__":
    encoded = json.dumps(generate(), indent=2, sort_keys=True) + "\n"
    if len(sys.argv) == 2:
        Path(sys.argv[1]).write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
