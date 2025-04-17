import numpy as np
import lmfit
import sigfig


def formatted_round(value, uncertainty):
    if uncertainty is None:
        return sigfig.round(value, style='Drake', sep='external_brackets', spacer='')
    else:
        return sigfig.round(value, uncertainty, style='Drake', sep='external_brackets', spacer='')

def GaussianFit(counts: list, centers: list,
                region_markers: list, peak_markers: list = [], background_markers: list = [],
                equal_sigma: bool = True, free_position: bool = True,                 
                background_params: dict = {'bg_type': 'linear', 
                                            'slope': (0, -np.inf, np.inf, 1.0, True), 
                                            'intercept': (0, -np.inf, np.inf, 0.0, True),
                                            'a': (0, -np.inf, np.inf, 0.0, True),
                                            'b': (0, -np.inf, np.inf, 1.0, True),
                                            'c': (0, -np.inf, np.inf, 0.0, True),
                                            'amplitude': (0, -np.inf, np.inf, 1.0, True),
                                            'decay': (0, -np.inf, np.inf, 1.0, True),
                                            'exponent': (0, -np.inf, np.inf, 1.0, True)
                                            }):
    """
    Performs a multi-Gaussian fit with an optional background model on a 1D histogram.

    This function fits one or more Gaussian peaks to a specified region of a 1D histogram,
    optionally using background subtraction and energy assignments. It supports constraints
    such as fixing peak positions, enforcing equal widths (σ), and calculating physical quantities 
    such as integrated area and cross sections.

    Parameters:
    ----------
    region_markers : tuple of float
        Two values defining the fitting region boundaries (in channel or calibrated units).
    peak_markers : list of float, optional
        Initial guesses for peak positions. If empty, the function guesses based on the maximum bin.
    background_markers : list of tuple, optional
        List of (start, end) ranges used to fit the background model. If empty, uses the full region.
    background_params : dict
        Dictionary specifying the background model type and its initial parameters.
        Supported types: 'linear', 'quadratic', 'exponential', 'powerlaw', or None.
    equal_sigma : bool, default=True
        If True, all Gaussians share the same sigma (FWHM). If False, each peak can vary independently.
    free_position : bool, default=True
        If True, allows peak positions to vary during fitting. If False, positions are fixed at initial guesses.
    print_info : bool, default=True
        If True, prints background and final fit reports to stdout.

    Notes:
    ------
    - The fit includes background + sum of Gaussian peaks.
    """

    # ensure the edges is the same length as counts + 1
    if len(centers) != len(counts):
        raise ValueError("The length of edges must be one more than the length of counts.")
    
    centers = np.array(centers)
    counts = np.array(counts)
    
    bin_width = centers[1] - centers[0]
    
    # Ensure there are 2 region markers
    if len(region_markers) != 2:
        raise ValueError("Region markers must have exactly 2 values.")
    
    # sort the region markers
    region_markers = sorted(region_markers)

    # enrsure there are only 2 region markers
    if len(region_markers) != 2:
        raise ValueError("Region markers must have exactly 2 values.")
    
    # Extract fitting region
    region_mask = (centers >= region_markers[0]) & (centers <= region_markers[1])

    x_data = centers[region_mask]
    y_data = counts[region_mask]

    # If there is not a peak marker, set it to the max bin value in the region
    if len(peak_markers) == 0:
        # find the bin with the max value in the region
        max_bin_idx = np.argmax(y_data)
        peak_markers = [x_data[max_bin_idx]]

    # sort the peak markers
    peak_markers = sorted(peak_markers)

    # Remove any peak markers that are outside the region
    peak_markers = [peak for peak in peak_markers if peak >= region_markers[0] and peak <= region_markers[1]]

    bg_type = background_params.get('bg_type', 'linear')
    
    if bg_type == 'linear':
        bg_model = lmfit.models.LinearModel(prefix='bg_')
        params = bg_model.make_params(slope=background_params['slope'][3], intercept=background_params['intercept'][3])
        params['bg_slope'].set(vary=background_params['slope'][4])
        params['bg_intercept'].set(vary=background_params['intercept'][4])
    elif bg_type == 'quadratic':
        bg_model = lmfit.models.QuadraticModel(prefix='bg_')
        params = bg_model.make_params(a=background_params['a'][3], b=background_params['b'][3], c=background_params['c'][3])
        params['bg_a'].set(vary=background_params['a'][4])
        params['bg_b'].set(vary=background_params['b'][4])
        params['bg_c'].set(vary=background_params['c'][4])
    elif bg_type == 'exponential':
        bg_model = lmfit.models.ExponentialModel(prefix='bg_')
        params = bg_model.make_params(amplitude=background_params['amplitude'][3], decay=background_params['decay'][3])
        params['bg_amplitude'].set(vary=background_params['amplitude'][4])
        params['bg_decay'].set(vary=background_params['decay'][4])
    elif bg_type == 'powerlaw':
        bg_model = lmfit.models.PowerLawModel(prefix='bg_')
        params = bg_model.make_params(amplitude=background_params['amplitude'][3], exponent=background_params['exponent'][3])
        params['bg_amplitude'].set(vary=background_params['amplitude'][4])
        params['bg_exponent'].set(vary=background_params['exponent'][4])
    elif bg_type == "None":
        bg_model = lmfit.models.ConstantModel(prefix='bg_')
        params = bg_model.make_params(c=0)
        params['bg_c'].set(vary=False)
    else:
        raise ValueError("Unsupported background model")
    
    # Fit the background model to the data of the background markers before fitting the peaks
    if len(background_markers) == 0:
        # put marker at the start and end of the region
        background_markers = [(region_markers[0]-bin_width, region_markers[0]), (region_markers[1], region_markers[1]+bin_width)]

    bg_x = []
    bg_y = []
    for bg_start, bg_end in background_markers:
        # sort the background markers
        bg_start, bg_end = sorted([bg_start, bg_end])

        bg_mask = (centers >= bg_start) & (centers <= bg_end)
        bg_x.extend(centers[bg_mask])
        bg_y.extend(counts[bg_mask])

    bg_x = np.array(bg_x)
    bg_y = np.array(bg_y)
    
    bg_result = bg_model.fit(bg_y, params, x=bg_x)

    # print intial parameter guesses
    print("\nInitial Background Parameter Guesses:")
    params.pretty_print()

    # print fit report
    print("\nBackground Fit Report:")
    print(bg_result.fit_report())

    # **Adjust background parameters based on their errors**
    for param in bg_result.params:
        params[param].set(value=bg_result.params[param].value, vary=False)

    # Add background model to overall model
    model = bg_model

    # Estimate sigma
    # **Find the peak marker with the highest bin count**
    peak_max_idx = np.argmax([y_data[np.abs(x_data - peak).argmin()] for peak in peak_markers])
    peak_with_max_count = peak_markers[peak_max_idx]

    # **Estimate sigma using FWHM method**
    def estimate_sigma(x_data, y_data, peak):
        peak_idx = np.abs(x_data - peak).argmin()
        peak_height = y_data[peak_idx]
        half_max = peak_height / 2

        # Find indices where y is closest to half the peak height
        left_idx = np.where(y_data[:peak_idx] <= half_max)[0]
        right_idx = np.where(y_data[peak_idx:] <= half_max)[0] + peak_idx

        if len(left_idx) == 0 or len(right_idx) == 0:
            return (x_data[1] - x_data[0]) * 2  # Fallback: Use bin width * 2

        left_fwhm = x_data[left_idx[-1]]
        right_fwhm = x_data[right_idx[0]]

        fwhm = right_fwhm - left_fwhm
        return max(fwhm / 2.3548, (x_data[1] - x_data[0]) * 2)  # Convert FWHM to sigma

    # **Get the estimated sigma from the strongest peak**
    estimated_sigma = estimate_sigma(x_data, y_data, peak_with_max_count)

    # **Estimate Amplitude for Each Peak**
    estimated_amplitude = []
    for peak in peak_markers:
        # Find closest bin index
        closest_idx = np.abs(x_data - peak).argmin()
        height = y_data[closest_idx]

        if bg_result is not None:
            # Estimate background contribution at this point if there is an background model
            bg_at_peak = bg_result.eval(x=peak)
        else:
            bg_at_peak = 0
        
        # Subtract background to get height
        adjusted_height = height - bg_at_peak
        estimated_amplitude.append(adjusted_height * estimated_sigma/ 0.3989423)

    # Add Gaussian peaks
    peak_markers = sorted(peak_markers)
    for i, peak in enumerate(peak_markers):
        # g = lmfit.Model(gaussian, prefix=f'g{i}_')
        g = lmfit.models.GaussianModel(prefix=f'g{i}_')
        model += g

        params.update(g.make_params(amplitude=estimated_amplitude[i], mean=peak, sigma=estimated_sigma))

        if equal_sigma and i > 0:
            params[f'g{i}_sigma'].set(expr='g0_sigma')
        else:
            params[f'g{i}_sigma'].set(min=0)

        params.add(f'g{i}_area', expr=f'g{i}_amplitude / {bin_width}')
        params[f"g{i}_area"].set(min=0)  # Use estimated area

        if not free_position:
            params[f'g{i}_center'].set(vary=False)

        if len(peak_markers) == 1:
            params[f'g{i}_center'].set(value=peak, min=region_markers[0], max=region_markers[1])
        else:
            # Default to using neighboring peaks
            prev_peak = region_markers[0] if i == 0 else peak_markers[i - 1]
            next_peak = region_markers[1] if i == len(peak_markers) - 1 else peak_markers[i + 1]

            # Calculate distance to previous and next peaks
            prev_dist = abs(peak - prev_peak)
            next_dist = abs(peak - next_peak)

            # Adjust min/max based 1 sigma of peak
            sigma_range = 1

            min_val = prev_peak if prev_dist <= sigma_range * estimated_sigma else peak - sigma_range * estimated_sigma
            max_val = next_peak if next_dist <= sigma_range * estimated_sigma else peak + sigma_range * estimated_sigma

            # Ensure bounds are within the region
            min_val = max(region_markers[0], min_val)
            max_val = min(region_markers[1], max_val)

            params[f'g{i}_center'].set(value=peak, min=min_val, max=max_val)

    # Fit the model to the data
    result = model.fit(y_data, params, x=x_data)

    # Print initial parameter guesses
    print("\nInitial Parameter Guesses:")
    params.pretty_print()

    # Print fit report
    print("\nFit Report:")

    fit_report = result.fit_report()
    print(fit_report)

    # Extract Gaussian and background parameters
    gaussian_params = []
    for i in range(len(peak_markers)):
        amplitude = float(result.params[f'g{i}_amplitude'].value)
        amplitude_uncertainty = result.params[f'g{i}_amplitude'].stderr or 0.0
        mean = float(result.params[f'g{i}_center'].value)
        mean_uncertainty = result.params[f'g{i}_center'].stderr or 0.0
        sigma = float(result.params[f'g{i}_sigma'].value)
        sigma_uncertainty = result.params[f'g{i}_sigma'].stderr or 0.0
        fwhm = float(result.params[f'g{i}_fwhm'].value)
        fwhm_uncertainty = result.params[f'g{i}_fwhm'].stderr or 0.0
        area = float(result.params[f'g{i}_area'].value)
        area_uncertainty = result.params[f'g{i}_area'].stderr or 0.0

        gaussian_params.append((
            amplitude, amplitude_uncertainty, mean, mean_uncertainty,
            sigma, sigma_uncertainty, fwhm, fwhm_uncertainty, area, area_uncertainty
        ))

        # Extract background parameters
        background_params = []
        if bg_type != 'None':
            for key in result.params:
                if 'bg_' in key:
                    value = float(result.params[key].value)
                    uncertainty = result.params[key].stderr or 0.0
                    background_params.append((key, value, uncertainty))

        # Create smooth fit line
        x_data_line = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
        y_data_line = result.eval(x=x_data_line)

    # save the fit result to a temp file
    save_modelresult(result, 'temp_fit.sav')

    return gaussian_params, background_params, x_data_line, y_data_line, fit_report

def load_result(filename: str):
    """
    Load a saved lmfit model result from a file.
    """
    result = load_modelresult(filename)

    params = result.params

    peak_markers = []
    for key in params:
        if 'g' in key and '_center' in key:
            peak_markers.append(params[key].value)

    x_min = result.userkws['x'].min()
    x_max = result.userkws['x'].max()
    x_data = np.linspace(x_min, x_max, 1000)

    # Print initial parameter guesses
    print("\nInitial Parameter Guesses:")
    params.pretty_print()

    # Print fit report
    print("\nFit Report:")

    fit_report = result.fit_report()
    print(fit_report)

    # Extract Gaussian and background parameters
    gaussian_params = []
    for i in range(len(peak_markers)):
        amplitude = float(result.params[f'g{i}_amplitude'].value)
        amplitude_uncertainty = result.params[f'g{i}_amplitude'].stderr or 0.0
        mean = float(result.params[f'g{i}_center'].value)
        mean_uncertainty = result.params[f'g{i}_center'].stderr or 0.0
        sigma = float(result.params[f'g{i}_sigma'].value)
        sigma_uncertainty = result.params[f'g{i}_sigma'].stderr or 0.0
        fwhm = float(result.params[f'g{i}_fwhm'].value)
        fwhm_uncertainty = result.params[f'g{i}_fwhm'].stderr or 0.0
        area = float(result.params[f'g{i}_area'].value)
        area_uncertainty = result.params[f'g{i}_area'].stderr or 0.0
        uuid = result.params.get(f'g{i}_uuid', None)

        gaussian_params.append((
            amplitude, amplitude_uncertainty, mean, mean_uncertainty,
            sigma, sigma_uncertainty, fwhm, fwhm_uncertainty, area, area_uncertainty, uuid
        ))

        # Extract background parameters
        background_params = []
        # if bg_type != 'None':
        for key in result.params:
            if 'bg_' in key:
                value = float(result.params[key].value)
                uncertainty = result.params[key].stderr or 0.0
                background_params.append((key, value, uncertainty))

        # Create smooth fit line
        x_data_line = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
        y_data_line = result.eval(x=x_data_line)

    # save the fit result to a temp file
    save_modelresult(result, 'temp_fit.sav')

    return gaussian_params, background_params, x_data_line, y_data_line, fit_report

def Add_UUID_to_Result(file_path: str, peak_number: int, uuid: int):
    # load the result
    result = load_modelresult(file_path)

    # print fit report
    print("\nPre Fit Report:")
    fit_report = result.fit_report()
    print(fit_report)

    # Add the UUID to the result
    if f"g{peak_number}_uuid" not in result.params:
        result.params.add(f"g{peak_number}_uuid", value=uuid, vary=False)
    else:
        # update the uuid
        result.params[f"g{peak_number}_uuid"].set(value=uuid, vary=False)

    # print fit report
    print("\nPost Fit Report:")
    fit_report = result.fit_report()
    print(fit_report)

    # save the fit result to a temp file
    save_modelresult(result, 'temp_fit.sav')

    return fit_report

import polars as pl
import matplotlib.pyplot as plt
from lmfit.model import load_modelresult, save_modelresult

import json

file = "/Users/alconley/OneDrive - Florida State University/2025_02_ICESPICE_Demonstrator/150Nd_p_t_148Nd/150Nd_pt_148Nd_Data/33deg_10.4kG/150Nd_pt_148Nd_33deg_10.4kG_run_162_164_179.parquet"

df = pl.read_parquet(file)

counts, edges = np.histogram(df["Xavg"], bins=600, range=(-300, 300))

centers = 0.5 * (edges[:-1] + edges[1:])

fig, axs = plt.subplots(1, 1, figsize=(10, 5))

axs.stairs(counts, edges, color='black', linewidth=0.5)

# _,_,_,_,_,result = GaussianFit(counts, centers, region_markers=[145, 155], peak_markers=[], background_params={'bg_type': "None"})

# save_modelresult(result, 'gauss_modelresult.sav')

# result = load_modelresult('gauss_modelresult.sav')
# # Provide the function in a dictionary
# funcdefs = {'gaussian': gaussian}

# result = load_modelresult("/Users/alconley/Downloads/fit_result.txt")


def plot_result(result, axs):
    # get the min x and max x values from the fit
    x_values = result.userkws['x']

    min_x = min(x_values)
    max_x = max(x_values)

    comp_x = np.linspace(min_x, max_x, 1000)
    comp_y = result.eval(x=comp_x)
    axs.plot(comp_x, comp_y, color='purple', linewidth=0.5)

    # Evaluate and plot individual components
    comps = result.eval_components(x=comp_x)
    for label, y in comps.items():
        if label != "bg_":
            color = "blue"
        else:
            color = "green"
        axs.plot(comp_x, y, linewidth=0.5, color=color)

with open("/Users/alconley/Downloads/all_lmfit_results.json") as f:
    result_dict = json.load(f)

fits = {}
for name, sav_text in result_dict.items():
    with open(f"{name}.sav", "w") as tmp:
        tmp.write(sav_text)
    result = load_modelresult(f"{name}.sav")

    # remove the temp file
    import os
    os.remove(f"{name}.sav")

    plot_result(result, axs)


# # plot the result
# x = np.linspace(edges[0], edges[-1], 1000)
# axs.plot(x, result.eval(x=x), 'r-', label='fit')
# axs.plot(x, result.eval_components(x=x), 'g--', label='components')
# axs.plot(x, result.eval(x=x) - result.eval_components(x=x), 'b--', label='background')

plt.show()