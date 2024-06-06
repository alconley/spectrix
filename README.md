# Nuclear Analysis Tool (NAT)

The Nuclear Analysis Tool (NAT) is a comprehensive software designed for nuclear data analysis. It provides functionalities for histogramming, Gaussian fitting, and interactive data visualization using `egui` and `egui_plot` crates.

## Features

- Create and manage histograms
- Fit Gaussian peaks
- Interactive data visualization
- Export and import fits
- Customizable plot settings

### Testing locally

Make sure you are using the latest version of stable rust by running `rustup update`. Rust is very easy to install on any computer. First, you'll need to install the Rust toolchain (compiler, cargo, etc). Go to the [Rust website](https://www.rust-lang.org/tools/install) and follow the instructions there.

Then clone the respository recursively

`git clone --recursive https://github.com/alconley/NAT.git`

`cargo run --release`

On Linux you need to first run:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev libgtk-3-dev`

On Fedora Rawhide you need to run:

`dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`

## 1D Histogram

### Fitting

Multiple gaussians can be fitted interactively similar to [hdtv](https://github.com/janmayer/hdtv).

**Keybinds**

- P: Add Marker
- B: Add Background Marker
- R: Add Region Marker
- -: Remove Marker Closest to Cursor
- Delete: Remove All Markers and Temp Fits
- G: Fit Background (Fit a linear background using the background markers)
- F: Fit Gaussians (Fit gaussians at the peak markers within a specified region with a linear background)
- S: Store Fit (Store the current fit as a permanent fit which can be saved and loaded later)
- I: Toggle Stats
- L: Toggle Log Y

The idea is to put region markers around you peak of intrest. If there are multiple peaks in between the region markers, add the centroid approximately with the peak markers. The background will be calculated with a linear line. To manually select the background data, add the background markers. If no background markers are supplied, background markers will be put at the region markers. The fits can be stored, and then the user can save/load them by right clicking on the plot and going into the fit menu.