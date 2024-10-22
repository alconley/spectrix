import lmfit
import numpy as np
import polars as pl
import matplotlib.pyplot as plt

def gaussian(x, amplitude, mean, sigma):
    return amplitude * np.exp(-(x - mean)**2 / (2 * sigma**2))

def MultipleGaussianFit(x_data: list, y_data: list, peak_markers: list, 
                        equal_sigma: bool = True, free_position: bool = True,
                        bg_type: str = 'linear', bg_initial_guesses: list = (0, 0, 0), bg_vary: list = (True, True, True)):
    
    # Initialize the model with or without a background based on the flag
    if bg_type == 'linear': 
        model = lmfit.models.LinearModel(prefix='bg_')
        params = model.make_params(slope=bg_initial_guesses[0], intercept=bg_initial_guesses[1])
        params['bg_slope'].set(vary=bg_vary[0])
        params['bg_intercept'].set(vary=bg_vary[1])
    elif bg_type == 'quadratic':
        model = lmfit.models.QuadraticModel(prefix='bg_')
        params = model.make_params(a=bg_initial_guesses[0], b=bg_initial_guesses[1], c=bg_initial_guesses[2])
        params['bg_a'].set(vary=bg_vary[0])
        params['bg_b'].set(vary=bg_vary[1])
        params['bg_c'].set(vary=bg_vary[2])
    elif bg_type == 'exponential':
        model = lmfit.models.ExponentialModel(prefix='bg_')
        params = model.make_params(amplitude=bg_initial_guesses[0], decay=bg_initial_guesses[1])
        params['bg_amplitude'].set(vary=bg_vary[0])
        params['bg_decay'].set(vary=bg_vary[1])
    elif bg_type == 'powerlaw':
        model = lmfit.models.PowerLawModel(prefix='bg_')
        params = model.make_params(amplitude=bg_initial_guesses[0], exponent=bg_initial_guesses[1])
        params['bg_amplitude'].set(vary=bg_vary[0])
        params['bg_exponent'].set(vary=bg_vary[1])
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
        
    params.update(first_gaussian.make_params(amplitude=1, mean=peak_markers[0], sigma=1))
    params['g0_sigma'].set(min=0)  # Initial constraint for the first Gaussian's sigma
    
    # Fix the center of the first Gaussian if free_position=False
    if not free_position:
        params['g0_mean'].set(vary=False)

    # Add additional Gaussians
    for i, peak in enumerate(peak_markers[1:], start=1):
        g = lmfit.Model(gaussian, prefix=f'g{i}_')
        model += g
        params.update(g.make_params(amplitude=1, mean=peak))

        # Set the sigma parameter depending on whether equal_sigma is True or False
        if equal_sigma:
            params[f'g{i}_sigma'].set(expr='g0_sigma')  # Constrain sigma to be the same as g1_sigma
        else:
            params[f'g{i}_sigma'].set(min=0)  # Allow different sigmas for each Gaussian

        params[f'g{i}_amplitude'].set(min=0)

        # Fix the center of the Gaussian if free_position=False
        if not free_position:
            params[f'g{i}_mean'].set(vary=False)

    # Fit the model to the data
    result = model.fit(y_data, params, x=x_data)

    print("\nFit Report:")
    print(result.fit_report())

    # Create a list of native Python float tuples for Gaussian parameters
    gaussian_params = []
    for i in range(len(peak_markers)):
        gaussian_params.append((
            float(result.params[f'g{i}_amplitude'].value),
            float(result.params[f'g{i}_amplitude'].stderr),
            float(result.params[f'g{i}_mean'].value),
            float(result.params[f'g{i}_mean'].stderr),
            float(result.params[f'g{i}_sigma'].value),
            float(result.params[f'g{i}_sigma'].stderr)
        ))

    # Create a list of native Python float tuples for Background parameters
    background_params = []
    if bg_type != 'None':
        for key in result.params:
            if 'bg_' in key:
                background_params.append((
                    key,  # Keep the parameter name
                    float(result.params[key].value),      # Convert the value to native Python float
                    float(result.params[key].stderr)      # Convert the uncertainty to native Python float
                ))

    # Create smooth fit line with plenty of data points
    x_data_line = np.linspace(x_data[0], x_data[-1], 5 * len(x_data))
    y_data_line = result.eval(x=x_data_line)

    fit_report = str(result.fit_report())

    return gaussian_params, background_params, x_data_line, y_data_line, fit_report

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
df = pl.read_parquet("/Users/alconley/Projects/ICESPICE/207Bi/exp_data/207Bi_noICESPICE_f9mm_g0mm_run_13.parquet")


df = df.with_columns([
    (pl.col("PIPS1000Energy") * 0.5395 + 2.5229).alias("PIPS1000EnergyCalibrated")
])


# Create a histogram from the x_data using np.histogram
counts, bin_edges = np.histogram(df["PIPS1000EnergyCalibrated"], bins=1200, range=(0, 1200))

# Get the bin centers
bin_centers = 0.5 * (bin_edges[:-1] + bin_edges[1:])

# Filter the bin centers and counts between 1018 and 1044
fit_mask = (bin_centers >= 520) & (bin_centers <= 600)
bin_centers_filtered = bin_centers[fit_mask]
counts_filtered = counts[fit_mask]


# Define the peak markers (this is where you mark the initial guesses for the peak positions)
peak_markers = [554, 567]  # Replace with actual peak guesses

# Fit the data
# gaussian_params, background_params, x_data_line, y_data_line = MultipleGaussianFit(bin_centers_filtered, counts_filtered, peak_markers, equal_sigma=True, free_position=False, bg_type='linear')

# slope = ("slope", -np.inf, np.inf, -2.0, False)
# intercept = ("intercept", -np.inf, np.inf, 0.0, False)
# LinearFit(bin_centers_filtered, counts_filtered, slope=slope, intercept=intercept)

ExponentialFit(bin_centers_filtered, counts_filtered)

# # 
# # Plot the original data and the fit
# plt.figure(figsize=(8, 6))

# # Plot the original data
# plt.step(bin_centers, counts, where="mid", label="Data")

# # Plot the Gaussian fit only in the selected range
# plt.plot(bin_centers_filtered, result.best_fit, color="red")

# # Add labels and legend
# plt.xlabel("PIPS1000Energy")
# plt.ylabel("Counts")
# plt.title("Gaussian Fit to Data")
# plt.legend()

# # Show the plot
# plt.show()