# Spectrix

Spectrix is a comprehensive software designed for nucelar spectrum analysis. It provides functionalities for histogramming, gaussian fitting, and interactive data visualization of 1D and 2D histograms using crates: `egui`, `egui-tiles`, `egui_plot`, and `polars`. Additionally, using uproot, you can view 1d and 2d root histograms.

### Running

Currently have tested this on Apple M3 Macbook Pro (Memory: 18 GB) running macOS Sonoma Version 14.6.1, Ubuntu 22.04.5 LTS and Windows 10. Both use python 3.13.

If you are using this on Windows, make sure your python is downloaded from [python.org](https://www.python.org/downloads/).

Make sure you are using the latest version of stable rust by running `rustup update`. Rust is very easy to install on any computer. First, you'll need to install the Rust toolchain (compiler, cargo, etc). Go to the [Rust website](https://www.rust-lang.org/tools/install) and follow the instructions there.

```sh
# For Linux/Mac OS

# Linux
sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev

# clone the repository
git clone https://github.com/alconley/spectrix.git

# Go into the spectrix folder
cd spectrix

# Create a Python virtual environment
python3 -m venv .venv

# Activate the virtual environment
source .venv/bin/activate

# Install the required python packages (lmfit and uproot)
pip install -r requirements.txt

# You might need to set the python environment/packages  (I need to do this on my mac)
# Set the PYO3_PYTHON environment variable to point to the Python in the virtual environment
export PYO3_PYTHON=$(pwd)/.venv/bin/python

# Set the PYTHONPATH to include the site-packages directory of the virtual environment
# Adjust the python version path
export PYTHONPATH=$(pwd)/.venv/lib/python3.*/site-packages

# Run the Rust project in release mode
cargo run --release

```
Tips if the program doesn't run:
`rustup update`
`cargo clean`
`cargo update`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev`

On Fedora Rawhide you need to run:

`dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`

## Overview

Spectrix program reads in `.parquet` files using the [Polars](https://docs.rs/polars/latest/polars/) crate. Personally, the .parquet files that I use are from [Eventbuilder](https://github.com/alconley/Eventbuilder). Here the parquet files are a dataframe (similar to a root tree) that stores the raw data as an f64. The histograms can be configured in the right panel in the ui. New column creation/cuts can also be done in the ui.

Additionally, the user can read in a 1D and 2D histograms from a root file using the python package: uproot. The user has to select "Root Files" in the Workspace for the files to appear in the gui. If there is an issure reading root files/additional requests let me know and I can try to add them. In the future, I would like to have the option to read in a root tree, and perform histogramming. However, for now, a root tree can be easily converted to a the parquet format using [hep-convert](https://hepconvert.readthedocs.io/en/latest/root_to_parquet.html).

## 1D Histograms

The goal was to create a very user-friendly UI that makes fitting peaks fun and enjoyable, unlike ROOT...

### Features

- Very Interactive UI
- Customizable elements
- Multiple Gaussian Fitting
- Different Background Models
- Rebinning Data
- Peak Finding

### Fitting
 
I opted to use python's [lmfit](https://lmfit.github.io/lmfit-py/builtin_models.html) to data in spectrix. Previously I used an awesome crate [varpro](https://github.com/geo-ant/varpro), however, I felt like I was reinventing the wheel for a lot. Therefore, I call python functions through the [pyo3](https://docs.rs/pyo3/latest/pyo3/). This adds extra dependencies and overhead but I think it is worth it to use the awesome fitting libray that lmfit has created while also making it easier for me to maintain/add new fitting functionalities to spectrix in the future.

Keybinds (cursor must be in the plot):

- P: Add Marker at cursor position
- B: Add Background Marker at cursor position
- R: Add Region Marker at cursor position
    - All markers can be moved by holding the middle mouse button on the center line dot and dragging it to the desired position.
    - Line settings are accessible in the context menu (right click) under the markers menu button.
- -: Remove Marker Closest to Cursor
- Delete: Remove All Markers and Temp Fits
- G: Fit Background
    - The background model can be changed in the Fits menu (right click on plot).
    - Models: Linear, Quadratic, Power Law, Exponential.
    - Initial guess, min, and max may need to be adjusted.
    - Data is evaluated at the bin center for the background marker.
- O: Peak find
    - Peak find settings are located in the context menu.
- F: Fit Gaussians
    - Settings and results can be found in the context menu.
    - Requires 2 region markers. Data will be evaluated between the markers.
    - Multiple Gaussians can be fitted together when multiple peak markers are between the region markers. By default, all peaks have the same standard deviation. This can be changed by checking the "Equal Standard Deviation" button. The position can also be locked if needed.
    - If no peak markers are between the region markers, the program will assume there is only 1 peak approximately at the max value in the data.
    - If there is no background fit when the fit button is clicked, the selected background model will be used and fitted. This can result in longer and potentially incorrect fits due to more parameters being fit. A fix would be adjusting the initial guess', min, and max value or fiting the background with the background markers.
    - The lmfit fit report, fit stats, and fit lines can be viewed in the Fits menu.
- S: Store Fit
    - Saves the fit.
    - Can save/load the fit to/from a file in the context menu.
- I: Toggle Stats
    - Display the mean, counts, and sigma on the histogram.
- L: Toggle Log Y

## 2D Histogram

### Features

- Very Interactive UI
- X and Y Projections
- Different Colormaps with that can be reversed, log norm, and adjustable Z range
- Easy to draw cut/gates
- Rebinning