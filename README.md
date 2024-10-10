# Spectrix

Spectrix is a comprehensive software designed for nucelar spectrum analysis. It provides functionalities for histogramming, Gaussian fitting, and interactive data visualization of 1D and 2D histograms using crates: `egui`, `egui-tiles`, `egui_plot`, and `polars`. Additionally, using uproot, you can view 1d and 2d root histograms.

### Running

Make sure you are using the latest version of stable rust by running `rustup update`. Rust is very easy to install on any computer. First, you'll need to install the Rust toolchain (compiler, cargo, etc). Go to the [Rust website](https://www.rust-lang.org/tools/install) and follow the instructions there.

Then clone the respository

`git clone https://github.com/alconley/spectrix.git`

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev`

On Fedora Rawhide you need to run:

`dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`

If you plan on using this program to view [root](https://root.cern) histograms, you will need to have python3 with the uproot package installed. If you are using a virtual environment (recommended), run these commands

```sh
# Create a Python virtual environment
python3 -m venv .venv

# Activate the virtual environment
source .venv/bin/activate

# Set the PYO3_PYTHON environment variable to point to the Python in the virtual environment
export PYO3_PYTHON=$(pwd)/.venv/bin/python

# Set the PYTHONPATH to include the site-packages directory of the virtual environment
export PYTHONPATH=$(pwd)/.venv/lib/python3.12/site-packages

# Run the Rust project in release mode
cargo run --release
```

## File Format

For version 1.0, this program reads in `.parquet` files using the [Polars](https://docs.rs/polars/latest/polars/) crate. Personally, the .parquet files that I use are from [Eventbuilder](https://github.com/alconley/Eventbuilder).

## 1D Histograms

The goal was to create a very user-friendly UI that makes fitting peaks fun and enjoyable, unlike ROOT...

### Features

- Very Interactive UI
- Customizable elements
- Multiple Gaussian Fitting
- Different Background Models
- Rebinning Data

### Fitting

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
    - Models: Any order polynomial (default is linear) and single/double exponential.
    - Data is evaluated at the bin center for the background marker.
- O: Peak find
    - Peak find settings are located in the context menu.
    - Use with caution, lol.
- F: Fit Gaussians
    - Settings and results can be found in the context menu.
    - Requires 2 region markers. Data will be evaluated between the markers.
    - Multiple Gaussians can be fitted together when multiple peak markers are between the region markers. By default, all peaks have the same standard deviation. This can be changed by checking the "free stddev" button. The position can also be locked if needed.
    - If no peak markers are between the region markers, the program will assume there is only 1 peak approximately at the max value in the data.
    - If there are no background markers when the fit button is clicked, background markers will be placed at the region markers.
- S: Store Fit
    - Saves the fit.
    - Can save/load the fit to/from a file in the context menu.
- I: Toggle Stats
    - Display the mean, counts, and sigma on the histogram.
- L: Toggle Log Y

![Fitting Example](assets/hist1d_fitting.gif)

#### Future Goals with Fitting

- Allow the user to plot different relationships (like FWHM vs Position, energy calibration, etc.)
- Pop out the fit panel into a new window
- More fit stats
- Open to any feedback

## 2D Histogram

### Features

- Very Interactive UI
- X and Y Projections
- Different Colormaps with that can be reversed, log norm, and adjustable Z range
- Easy to draw cut/gates
- Rebinning

### Cutting/Gating Data

A powerful feature of this program is the ability to filter data using cuts or gates on a 2D histogram. This is done by drawing a polygon around the region of interest, saving the file, and then loading it into the program through the side panel. When you calculate histograms with cuts, a boolean mask is created to filter the data in the underlying lazy frame based on the columns provided by the user.

#### Steps to Create a Cut/Gate

1. **Draw a Polygon**: Use the interactive UI to draw a polygon around the region of interest on the 2D histogram. This can be activated by right-clicking on the plot and selecting the "Add Cut" button. To place vertices of the polygon, click around the data of interest. Double-click the final point to complete the polygon. The vertices can be moved with the middle mouse button.

2. **Save and Load the Cut**: Once the polygon is drawn, save the cut somewhere on your computer. Load the cut in the info panel.

3. **Filter the Lazy Frame**: A boolean mask will be computed for the specified columns (the user must specify which columns to filter). The entire lazy frame will be filtered using this boolean mask. Multiple cuts can be used; just ensure they are activated.

Here is an example of filtering our data using a particle identification 2D histogram. In the example, I select all the data that corresponds to the protons.

![cut example](assets/cut_example.gif)

### Projecting Data

Another useful feature of the 2D histogram is the ability to project data along the X or Y axis to create 1D histograms. This allows for analysis of data distribution along a particular axis.

#### Steps to Create a Projection

1. **Activate Projection**: To create a projection, right-click on the 2D histogram and select "Add X Projection" (keybind: x) or "Add Y Projection" (keybind: y).

2. **Adjust Projection Lines**: Once the projection is activated, lines will appear on the histogram indicating the range of the projection. These lines can be moved using the middle mouse button to select the desired range.

3. **View and Use the Projection**: The projection will generate a 1D histogram based on the selected range. This 1D histogram can be used for further analysis, such as fitting peaks.
