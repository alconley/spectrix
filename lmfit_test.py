import lmfit
import numpy as np
import polars as pl
import matplotlib.pyplot as plt

def gaussian(x, amplitude, mean, sigma):
    return amplitude * np.exp(-(x - mean)**2 / (2 * sigma**2))

# Multiple Gaussian fitting function
def MultipleGaussianFit(x_data: list, y_data: list, peak_markers: list, equal_sigma:bool=True, free_position:bool=True, background_type:str='linear'):
    
    # Initialize the model with or without a background based on the flag
    if background_type == 'linear':
        model = lmfit.models.LinearModel(prefix='bg_')
        params = model.make_params(slope=0, intercept=0)  # Initial guesses for linear background
    elif background_type == 'quadratic':
        model = lmfit.models.QuadraticModel(prefix='bg_')
        params = model.make_params(a=0, b=1, c=0)  # Initial guesses for quadratic background
    elif background_type == 'exponential':
        model = lmfit.models.ExponentialModel(prefix='bg_')
        params = model.make_params(amplitude=1, decay=100)  # Initial guesses for exponential background
    elif background_type == 'powerlaw':
        model = lmfit.models.PowerLawModel(prefix='bg_')
        params = model.make_params(amplitude=1, exponent=-0.5)  # Initial guesses for power-law background
    elif background_type is None:
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
        print(f'Adding Gaussian {i} at {peak}')
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

    print(result.fit_report())

    # Create an array of (name, value, uncertainty) tuples
    param_array = [(param, result.params[param].value, result.params[param].stderr) for param in result.params]
    
    
    
    x_result = x_data
    y_result = result.best_fit
    
    composition_points = [x_data, result.best_fit]
    
    
    # Return the fit result
    return param_array, 

# Load the data using Polars
df = pl.read_parquet("/home/alconley/git_workspace/ICESPICE/207Bi/exp_data/207Bi_noICESPICE_f9mm_g0mm_run_13.parquet")

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
param = MultipleGaussianFit(bin_centers_filtered, counts_filtered, peak_markers, equal_sigma=True, free_position=False, background_type='powerlaw')

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