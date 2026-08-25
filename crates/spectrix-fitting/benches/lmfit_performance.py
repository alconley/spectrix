"""Warmed end-to-end lmfit benchmark used by the native speed gate."""

from __future__ import annotations

import json
import math
import sys
import time

import lmfit
import numpy as np
import scipy


EXPECTED = ("1.3.4", "2.5.2", "1.18.1")


def make_data(bin_count: int, peak_count: int):
    x = np.linspace(1.0, 11.0, bin_count)
    true_centers = np.array(
        [2.0 + 8.0 * (index + 1) / (peak_count + 1) for index in range(peak_count)]
    )
    markers = true_centers.copy()
    y = 3.0 + 0.18 * x
    for peak, center in enumerate(true_centers):
        amplitude = 75.0 + peak * 8.0
        sigma = 0.11 + peak * 0.006
        y += amplitude / (math.sqrt(2.0 * math.pi) * sigma) * np.exp(
            -0.5 * ((x - center) / sigma) ** 2
        )
    y += 0.015 * np.sin(np.arange(bin_count) * 0.17)
    return x, y, markers


def estimate_sigma(x, y, marker: float, bin_width: float) -> float:
    peak_index = int(np.abs(x - marker).argmin())
    half_maximum = y[peak_index] / 2.0
    left = np.where(y[:peak_index] <= half_maximum)[0]
    right = np.where(y[peak_index:] <= half_maximum)[0] + peak_index
    if len(left) and len(right):
        return max((x[right[0]] - x[left[-1]]) / 2.3548, bin_width * 2.0)
    return bin_width * 2.0


def center_bounds(index: int, markers, sigma: float):
    if len(markers) == 1:
        return 1.0, 11.0
    marker = markers[index]
    previous = 1.0 if index == 0 else markers[index - 1]
    following = 11.0 if index + 1 == len(markers) else markers[index + 1]
    minimum = previous if abs(marker - previous) <= sigma else marker - sigma
    maximum = following if abs(following - marker) <= sigma else marker + sigma
    return max(minimum, 1.0), min(maximum, 11.0)


def fit_case(x, y, markers):
    background_mask = ((x >= 1.0) & (x <= 1.6)) | ((x >= 10.4) & (x <= 11.0))
    background = lmfit.models.LinearModel(prefix="bg_")
    background_result = background.fit(
        y[background_mask], background.make_params(slope=0.0, intercept=0.0), x=x[background_mask]
    )
    parameters = background_result.params.copy()
    for parameter in parameters.values():
        parameter.set(vary=False)

    bin_width = 10.0 / (len(x) - 1)
    sigmas = [estimate_sigma(x, y, marker, bin_width) for marker in markers]
    peak_heights = [y[int(np.abs(x - marker).argmin())] for marker in markers]
    strongest_sigma = sigmas[int(np.argmax(peak_heights))]
    model = background
    for index, marker in enumerate(markers):
        gaussian = lmfit.models.GaussianModel(prefix=f"g{index}_")
        adjusted_height = peak_heights[index] - background_result.eval(x=marker)
        amplitude = max(adjusted_height * strongest_sigma / 0.3989423, 0.0)
        parameters.update(
            gaussian.make_params(amplitude=amplitude, center=marker, sigma=sigmas[index])
        )
        parameters[f"g{index}_amplitude"].set(min=0.0)
        parameters[f"g{index}_sigma"].set(min=0.0)
        minimum, maximum = center_bounds(index, markers, strongest_sigma)
        parameters[f"g{index}_center"].set(min=minimum, max=maximum, vary=False)
        parameters.add(f"g{index}_area", expr=f"g{index}_amplitude / {bin_width!r}", min=0.0)
        model += gaussian

    result = model.fit(y, parameters, x=x)
    evaluation_x = np.linspace(x[0], x[-1], 50 * len(x))
    best_fit = result.eval(x=evaluation_x)
    component_values = result.eval_components(x=evaluation_x)
    uncertainty = result.eval_uncertainty(x=evaluation_x, sigma=1)
    if not hasattr(result, "dely_comps"):
        unavailable = [name for name, parameter in result.params.items() if parameter.stderr is None]
        raise RuntimeError(
            f"lmfit component covariance unavailable ({result.message}, ier={result.ier}, "
            f"covariance={result.covar is not None}) for parameters: {unavailable}"
        )
    total_band = (
        evaluation_x.copy(),
        best_fit.copy(),
        uncertainty.copy(),
        best_fit - uncertainty,
        best_fit + uncertainty,
    )
    component_bands = {
        name: (
            evaluation_x.copy(),
            values.copy(),
            result.dely_comps[name].copy(),
            values - result.dely_comps[name],
            values + result.dely_comps[name],
        )
        for name, values in component_values.items()
    }
    return result, total_band, component_bands


def benchmark() -> dict[str, float]:
    versions = (lmfit.__version__, np.__version__, scipy.__version__)
    if versions != EXPECTED:
        raise RuntimeError(f"parity environment is {versions}, expected {EXPECTED}")
    timings = {}
    for peak_count in (1, 3, 8):
        for bin_count in (512, 2048, 8192):
            x, y, markers = make_data(bin_count, peak_count)
            fit_case(x, y, markers)
            repeats = 5 if bin_count == 512 else 3 if bin_count == 2048 else 1
            started = time.perf_counter()
            for _ in range(repeats):
                fit_case(x, y, markers)
            timings[f"{peak_count}p_{bin_count}b"] = (
                (time.perf_counter() - started) * 1.0e6 / repeats
            )
    return timings


if __name__ == "__main__":
    json.dump(benchmark(), sys.stdout, sort_keys=True)
    print()
