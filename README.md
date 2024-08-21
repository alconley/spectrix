# general Nuclear Analysis Tool (gNAT)

The general Nuclear Analysis Tool (gNAT, pronounced /ɡnæt/, with a hard 'G' sound followed by 'nat) is a comprehensive software designed for nuclear data analysis. It provides functionalities for histogramming, Gaussian fitting, and interactive data visualization of 1D and 2D histograms using crates: `egui`, `egui-tiles`, `egui_plot`, and `polars`.

### Running locally

Make sure you are using the latest version of stable rust by running `rustup update`. Rust is very easy to install on any computer. First, you'll need to install the Rust toolchain (compiler, cargo, etc). Go to the [Rust website](https://www.rust-lang.org/tools/install) and follow the instructions there.

Then clone the respository

`git clone https://github.com/alconley/gNAT.git`

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev`

On Fedora Rawhide you need to run:

`dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`

## File Format

For version 1.0, this program reads in `.parquet` files using the [Polars](https://docs.rs/polars/latest/polars/) crate. Personally, the .parquet files that I use are from [Eventbuilder](https://github.com/alconley/Eventbuilder).

### About Parquet

Parquet is a columnar storage file format designed for efficient data storage and retrieval. Here are some of the benefits of using Parquet:

- **Efficient Storage**: Parquet files are highly compressed, which reduces storage costs and improves read and write performance.
- **Columnar Storage**: This format is optimized for querying and can read only the necessary columns, which speeds up data access.
- **Compatibility**: Parquet is compatible with many data processing frameworks, including Apache Spark, Apache Drill, and more.
- **Schema Evolution**: Parquet supports schema evolution, allowing you to add new columns to your dataset without breaking existing queries.

## Configuring Histogram Script

If someone is using this program outside of the [John D. Fox Lab](https://fsunuc.physics.fsu.edu/wiki/index.php/FSU_Fox%27s_Lab_Wiki) (i.e., the [SE-SPS](https://fsunuc.physics.fsu.edu/wiki/index.php/Split-Pole_Spectrograph) or [CeBrA](https://www.sciencedirect.com/science/article/abs/pii/S0168900223008185)), you will have to configure your histogram script. I currently have it set up so you can do a manual histogram script (example in `src/histogram_scripter/manual_histogram_script.rs`), or you have to configure the interactive histogrammer for your purposes. Both require some knowledge of how to code in Rust (mostly [polars](https://docs.rs/polars/latest/polars/) syntax), but I think it is pretty straightforward. The latter has the benefit of being able to change the binning and really explore your data without having to recompile.

To configure your interactive histogrammer UI:

1. Go to `src/histogram_scripter/configure_lazyframes.rs`.

   - Adjust the `add_columns_to_lazyframe` function to add extra columns to the main lazyframe.
   - Adjust the `filtered_lfs` function to declare what lazyframes the user can pick the data frame from in the UI. This is essentially where the user can filter the lazyframe with specific conditions.
   - Hard code the column names in the `main_column_names` function. This ensures the user never calls a column that doesn't exist.
   - Likewise, adjust the lazyframe names in the `main_lfs_names` function.

2. Configure auxiliary columns (Optional).

   - This option exists as an example of how a more interactive UI can be implemented for the interactive histogrammer. In the existing code, the auxiliary columns correspond to CeBrA. For these detectors, the time filtering condition and energy calibration values change with every experiment. So this enables the user to easily energy calibrate the data if needed.
   - Just like above, you have to configure the different column names, lazyframes, and the UI.


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

1. **Activate Projection**: To create a projection, right-click on the 2D histogram and select "Add X Projection" or "Add Y Projection".

2. **Adjust Projection Lines**: Once the projection is activated, lines will appear on the histogram indicating the range of the projection. These lines can be moved using the middle mouse button to select the desired range.

3. **View and Use the Projection**: The projection will generate a 1D histogram based on the selected range. This 1D histogram can be used for further analysis, such as fitting peaks.

![projection example](assets/projection_example.gif)

## Future Features

- Save/load all the histograms into some python program
- Command line interface / bashscripting