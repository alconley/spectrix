import lmfit
import numpy as np
import polars as pl
import matplotlib.pyplot as plt

def gaussian(x, amplitude, mean, sigma):
    return amplitude * np.exp(-(x - mean)**2 / (2 * sigma**2))


def gaussian(x, amplitude, mean, sigma):
    return amplitude * np.exp(-(x - mean)**2 / (2 * sigma**2))

import numpy as np
import lmfit

def MultipleGaussianFit(x_data: list, y_data: list, peak_markers: list, bin_width: float, 
                        equal_sigma: bool = True, free_position: bool = True,
                        background_params: dict = None):
    """
    Multiple Gaussian fit function with background model support.
    
    Parameters:
    - x_data, y_data: Lists of data points.
    - peak_markers: List of peak positions for the Gaussians.
    - equal_sigma: Whether to constrain all Gaussians to have the same sigma.
    - free_position: Whether to allow the positions of Gaussians to vary.
    - background_params: Dictionary containing background model type and parameters.
    """
    
    # Default background params if none are provided
    if background_params is None:
        background_params = {
            'bg_type': 'linear',
            'slope': ("slope", -np.inf, np.inf, 0.0, True),
            'intercept': ("intercept", -np.inf, np.inf, 0.0, True),
            'a': ("a", -np.inf, np.inf, 0.0, True),
            'b': ("b", -np.inf, np.inf, 0.0, True),
            'c': ("c", -np.inf, np.inf, 0.0, True),
            'exponent': ("exponent", -np.inf, np.inf, 0.0, True),
            'amplitude': ("amplitude", -np.inf, np.inf, 0.0, True),
            'decay': ("decay", -np.inf, np.inf, 0.0, True),
        }
    
    bg_type = background_params.get('bg_type', 'linear')
    slope = background_params.get('slope')
    intercept = background_params.get('intercept')
    a = background_params.get('a')
    b = background_params.get('b')
    c = background_params.get('c')
    amplitude = background_params.get('amplitude')
    exponent = background_params.get('exponent')
    decay = background_params.get('decay')

    # Initialize the model with or without a background based on bg_type
    if bg_type == 'linear': 
        model = lmfit.models.LinearModel(prefix='bg_')
        params = model.make_params(slope=slope[3], intercept=intercept[3])
        params['bg_slope'].set(min=slope[1], max=slope[2], value=slope[3], vary=slope[4])
        params['bg_intercept'].set(min=intercept[1], max=intercept[2], value=intercept[3], vary=intercept[4])
    elif bg_type == 'quadratic':
        model = lmfit.models.QuadraticModel(prefix='bg_')
        params = model.make_params(a=a[3], b=b[3], c=c[3])
        params['bg_a'].set(min=a[1], max=a[2], value=a[3], vary=a[4])
        params['bg_b'].set(min=b[1], max=b[2], value=b[3], vary=b[4])
        params['bg_c'].set(min=c[1], max=c[2], value=c[3], vary=c[4])
    elif bg_type == 'exponential':
        model = lmfit.models.ExponentialModel(prefix='bg_')
        params = model.make_params(amplitude=amplitude[3], decay=decay[3])
        params['bg_amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
        params['bg_decay'].set(min=decay[1], max=decay[2], value=decay[3], vary=decay[4])
    elif bg_type == 'powerlaw':
        model = lmfit.models.PowerLawModel(prefix='bg_')
        params = model.make_params(amplitude=amplitude[3], exponent=exponent[3])
        params['bg_amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
        params['bg_exponent'].set(min=exponent[1], max=exponent[2], value=exponent[3], vary=exponent[4])
    elif bg_type is None:
        model = None
        params = lmfit.Parameters()
    else:
        raise ValueError("Unsupported background model")

    first_gaussian = lmfit.Model(gaussian, prefix=f'g0_')

    if model is None:
        model = first_gaussian
    else:
        model += first_gaussian
        
    if len(peak_markers) == 0:
        peak_markers = [x_data[np.argmax(y_data)]]

    peak_markers = sorted(peak_markers)  # sort the peak markers in ascending order

    estimated_amplitude = 1000
    estimated_sigma = 10

    params.update(first_gaussian.make_params(amplitude=estimated_amplitude, mean=peak_markers[0], sigma=estimated_sigma))
    params['g0_sigma'].set(min=0)  # Initial constraint for the first Gaussian's sigma
    params[f"g0_amplitude"].set(min=0)

    params.add(f'g0_fwhm', expr=f'2.35482 * g0_sigma')  # FWHM = 2 * sqrt(2 * ln(2)) * sigma
    params[f"g0_fwhm"].set(min=0)

    params.add(f'g0_area', expr=f'g0_amplitude * sqrt(2 * pi) * g0_sigma / {bin_width}')  # Area under the Gaussian
    params[f"g0_area"].set(min=0)

    if not free_position:
        params['g0_mean'].set(vary=False)

    params['g0_mean'].set(min=x_data[0], max=peak_markers[1] if len(peak_markers) > 1 else x_data[-1])

    # Add additional Gaussians
    for i, peak in enumerate(peak_markers[1:], start=1):
        g = lmfit.Model(gaussian, prefix=f'g{i}_')
        model += g

        estimated_amplitude = 1000
        params.update(g.make_params(amplitude=estimated_amplitude, mean=peak, sigma=10))

        min_mean = peak_markers[i-1]
        max_mean = peak_markers[i+1] if i + 1 < len(peak_markers) else x_data[-1]
        params[f'g{i}_mean'].set(min=min_mean, max=max_mean)

        params.add(f'g{i}_fwhm', expr=f'2.35482 * g{i}_sigma')
        params[f"g{i}_fwhm"].set(min=0)

        params.add(f'g{i}_area', expr=f'g{i}_amplitude * sqrt(2 * pi) * g{i}_sigma / {bin_width}')
        params[f"g{i}_area"].set(min=0)

        if equal_sigma:
            params[f'g{i}_sigma'].set(expr='g0_sigma')
        else:
            params[f'g{i}_sigma'].set(min=0)

        params[f'g{i}_amplitude'].set(min=0)

        if not free_position:
            params[f'g{i}_mean'].set(vary=False)

    # Fit the model to the data
    result = model.fit(y_data, params, x=x_data)

    print("\nInitial Parameter Guesses:")
    params.pretty_print()

    print("\nFit Report:")
    print(result.fit_report())

    # Extract Gaussian and background parameters
    gaussian_params = []
    for i in range(len(peak_markers)):
        amplitude = float(result.params[f'g{i}_amplitude'].value)
        amplitude_uncertainty = result.params[f'g{i}_amplitude'].stderr or 0.0
        mean = float(result.params[f'g{i}_mean'].value)
        mean_uncertainty = result.params[f'g{i}_mean'].stderr or 0.0
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

    fit_report = str(result.fit_report())

    return gaussian_params, background_params, x_data_line, y_data_line, fit_report, result


# Multiple Gaussian fitting function
def LinearFit(x_data: list, y_data: list, slope: list = ("slope", -np.inf, np.inf, 0.0, True), intercept = ("intercept", -np.inf, np.inf, 0.0, True)):
    import lmfit
    import numpy as np
    
    # params = slope=[name, min, max, initial_guess, vary], intercept=[name, min, max, initial_guess, vary]
    
    model = lmfit.models.LinearModel()
    params = model.make_params(slope=slope[3], intercept=intercept[3])
    params['slope'].set(min=slope[1], max=slope[2], value=slope[3], vary=slope[4])
    params['intercept'].set(min=intercept[1], max=intercept[2], value=intercept[3], vary=intercept[4])

    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    slope = float(result.params['slope'].value)
    slope_err = result.params['slope'].stderr

    if slope_err is None:
        slope_err = float(0.0)
    else:
        slope_err = float(slope_err)

    intercept = float(result.params['intercept'].value)
    
    intercept_err = result.params['intercept'].stderr
    if intercept_err is None:
        intercept_err = float(0.0)
    else:
        intercept_err = float(intercept_err)

    print(f"Slope: {slope} ± {slope_err}")
    print(f"Intercept: {intercept} ± {intercept_err}")


    params = [
        ('slope', slope, slope_err),
        ('intercept', intercept, intercept_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    return params, x, y, fit_report

def QuadraticFit(x_data: list, y_data: list, a: list = ("a", -np.inf, np.inf, 0.0, True), b = ("b", -np.inf, np.inf, 0.0, True), c: list = ("a", -np.inf, np.inf, 0.0, True),):    
    # params = [name, min, max, initial_guess, vary]
    
    model = lmfit.models.QuadraticModel()
    params = model.make_params(a=a[3], b=b[3], c=c[3])
    params['a'].set(min=a[1], max=a[2], value=a[3], vary=a[4])
    params['b'].set(min=b[1], max=b[2], value=b[3], vary=b[4])
    params['c'].set(min=c[1], max=c[2], value=c[3], vary=c[4])
    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    a = float(result.params['a'].value)
    a_err = result.params['a'].stderr
    if a_err is None:
        a_err = float(0.0)
    else:
        a_err = float(a_err)

    b = float(result.params['b'].value)
    b_err = result.params['b'].stderr
    if b_err is None:
        b_err = float(0.0)
    else:
        b_err = float(b_err)

    c = float(result.params['c'].value)
    c_err = result.params['c'].stderr
    if c_err is None:
        c_err = float(0.0)
    else:
        c_err = float(c_err)


    params = [
        ('a', a, a_err),
        ('b', b, b_err),
        ('c', c, c_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    return params, x, y, fit_report

def PowerLawFit(x_data: list, y_data: list, amplitude: list = ("amplitude", -np.inf, np.inf, 0.0, True), exponent = ("exponent", -np.inf, np.inf, 0.0, True)):    
    # params = [name, min, max, initial_guess, vary]
    
    model = lmfit.models.PowerLawModel()
    params = model.make_params(amplitude=amplitude[3], exponent=exponent[3])
    params['amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
    params['exponent'].set(min=exponent[1], max=exponent[2], value=exponent[3], vary=exponent[4])
    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    amplitude = float(result.params['amplitude'].value)
    amplitude_err = result.params['amplitude'].stderr
    if amplitude_err is None:
        amplitude_err = float(0.0)
    else:
        amplitude_err = float(amplitude_err)
    
    exponent = float(result.params['exponent'].value)
    exponent_err = result.params['exponent'].stderr
    if exponent_err is None:
        exponent_err = float(0.0)
    else:
        exponent_err = float(exponent_err)

    params = [
        ('amplitude', amplitude, amplitude_err),
        ('exponent', exponent, exponent_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    return params, x, y, fit_report

def ExponentialFit(x_data: list, y_data: list, amplitude: list = ("amplitude", -np.inf, np.inf, 0.0, True), decay = ("decay", -np.inf, np.inf, 0.0, True)):    
    # params = [name, min, max, initial_guess, vary]
    
    model = lmfit.models.ExponentialModel()
    params = model.make_params(amplitude=amplitude[3], decay=decay[3])
    params['amplitude'].set(min=amplitude[1], max=amplitude[2], value=amplitude[3], vary=amplitude[4])
    params['decay'].set(min=decay[1], max=decay[2], value=decay[3], vary=decay[4])
    result = model.fit(y_data, params, x=x_data)

    print(result.fit_report())

    # Extract Parameters
    amplitude = float(result.params['amplitude'].value)
    amplitude_err = result.params['amplitude'].stderr
    if amplitude_err is None:
        amplitude_err = float(0.0)
    else:
        amplitude_err = float(amplitude_err)
    
    decay = float(result.params['decay'].value)
    decay_err = result.params['decay'].stderr
    if decay_err is None:
        decay_err = float(0.0)
    else:
        decay_err = float(decay_err)

    params = [
        ('amplitude', amplitude, amplitude_err),
        ('decay', decay, decay_err)
    ]

    x = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y = result.eval(x=x)

    fit_report = str(result.fit_report())

    return params, x, y, fit_report


# Load the data using Polars
# df = pl.read_parquet("/Users/alconley/Projects/ICESPICE/207Bi/exp_data/207Bi_noICESPICE_f9mm_g0mm_run_13.parquet")
df = pl.read_parquet("../ICESPICE/207Bi/exp_data/207Bi_noICESPICE_f9mm_g0mm_run_13.parquet")


df = df.with_columns([
    (pl.col("PIPS1000Energy") * 0.5395 + 2.5229).alias("PIPS1000EnergyCalibrated")
])


# Create a histogram from the x_data using np.histogram
counts, bin_edges = np.histogram(df["PIPS1000EnergyCalibrated"], bins=1200, range=(0, 1200))

# Get the bin centers
bin_centers = 0.5 * (bin_edges[:-1] + bin_edges[1:])

# Filter the bin centers and counts between 1018 and 1044
fit_mask = (bin_centers >= 400) & (bin_centers <= 600)
bin_centers_filtered = bin_centers[fit_mask]
counts_filtered = counts[fit_mask]


# Define the peak markers (this is where you mark the initial guesses for the peak positions)
peak_markers = [475, 550, 575]  # Replace with actual peak guesses

# Fit the data
gaussian_params, background_params, x_data_line, y_data_line, fit_report, result = MultipleGaussianFit(bin_centers_filtered, counts_filtered, peak_markers, 1.0, equal_sigma=True, free_position=True)

# slope = ("slope", -np.inf, np.inf, -2.0, False)
# intercept = ("intercept", -np.inf, np.inf, 0.0, False)
# LinearFit(bin_centers_filtered, counts_filtered, slope=slope, intercept=intercept)

# ExponentialFit(bin_centers_filtered, counts_filtered)

# # 
# # Plot the original data and the fit
plt.figure(figsize=(8, 6))

# Plot the original data
plt.step(bin_centers, counts, where="mid", label="Data")

# Plot the Gaussian fit only in the selected range
plt.plot(bin_centers_filtered, result.best_fit, color="red")

# Add labels and legend
plt.xlabel("PIPS1000Energy")
plt.ylabel("Counts")
plt.title("Gaussian Fit to Data")
plt.legend()

# Show the plot
plt.show()