import numpy as np
import uproot

def write_histograms(output_file, hist1d_data, hist2d_data):
    """
    Writes 1D and 2D histograms to a ROOT file.

    Parameters:
        output_file (str): Path to the output ROOT file.
        hist1d_data (list): List of tuples for 1D histograms. Each tuple contains:
            - name (str): Histogram name.
            - title (str): Histogram title.
            - bins (list of int): Bin counts.
            - underflow (int): Underflow count.
            - overflow (int): Overflow count.
            - range (tuple): Range of the histogram as (min, max).
        hist2d_data (list): List of tuples for 2D histograms. Each tuple contains:
            - name (str): Histogram name.
            - title (str): Histogram title.
            - bins (list of list of int): Bin counts (2D array).
            - range_x (tuple): Range of the X-axis as (min, max).
            - range_y (tuple): Range of the Y-axis as (min, max).
    """
    with uproot.recreate(output_file) as file:
        for name, title, bins, underflow, overflow, range in hist1d_data:
            # Create bin edges for the histogram
            bin_edges = np.linspace(range[0], range[1], len(bins) + 1)
            
            # Include underflow and overflow in the data array
            data = np.array([underflow] + bins + [overflow], dtype=np.float32)
            bins_array = np.array(bins, dtype=np.float32)  # Convert bins to numpy array

            # Define fXaxis using to_TAxis with positional arguments
            fXaxis = uproot.writing.identify.to_TAxis(
                fName="xaxis",         # Temporary name for the X-axis
                fTitle="",       # Title of the X-axis
                fNbins=len(bins),      # Number of bins
                fXmin=range[0],       # Minimum X-axis value
                fXmax=range[1],       # Maximum X-axis value
                fXbins=bin_edges       # Bin edges
            )

            # Calculate metadata
            fEntries = float(np.sum(bins))
            fTsumw = float(np.sum(bins))
            fTsumw2 = float(np.sum(bins_array**2))
            fTsumwx = float(np.sum(bins_array * bin_edges[:-1]))
            fTsumwx2 = float(np.sum(bins_array * bin_edges[:-1]**2))
            fSumw2 = None

            # Write the histogram using uproot.writing.identify.to_TH1x
            file[name] = uproot.writing.identify.to_TH1x(
                fName=None,
                fTitle=title,
                data=data,
                fEntries=fEntries,
                fTsumw=fTsumw,
                fTsumw2=fTsumw2,
                fTsumwx=fTsumwx,
                fTsumwx2=fTsumwx2,
                fSumw2=fSumw2,
                fXaxis=fXaxis
            )

            print(f"1D Histogram '{name}' written successfully.")
            
        # Write 2D histograms
        for name, title, bins, range_x, range_y in hist2d_data:
            bins = np.array(bins, dtype=np.float32)
            # Flatten the 2D array with added underflow/overflow bins
            bins_with_overflow = np.zeros((bins.shape[0] + 2, bins.shape[1] + 2), dtype=np.float32)
            bins_with_overflow[1:-1, 1:-1] = bins
            data = bins_with_overflow.flatten()

            x_bin_edges = np.linspace(range_x[0], range_x[1], bins.shape[1] + 1)
            y_bin_edges = np.linspace(range_y[0], range_y[1], bins.shape[0] + 1)

            fXaxis = uproot.writing.identify.to_TAxis(
                fName="xaxis",
                fTitle="X-axis",
                fNbins=bins.shape[1],
                fXmin=range_x[0],
                fXmax=range_x[1],
                fXbins=x_bin_edges
            )

            fYaxis = uproot.writing.identify.to_TAxis(
                fName="yaxis",
                fTitle="Y-axis",
                fNbins=bins.shape[0],
                fXmin=range_y[0],
                fXmax=range_y[1],
                fXbins=y_bin_edges
            )

            # Compute required statistical sums
            x_centers = (x_bin_edges[:-1] + x_bin_edges[1:]) / 2
            y_centers = (y_bin_edges[:-1] + y_bin_edges[1:]) / 2

            fTsumw = np.sum(bins)
            fTsumw2 = np.sum(bins**2)
            fTsumwx = np.sum(bins * x_centers[np.newaxis, :])
            fTsumwx2 = np.sum(bins * (x_centers[np.newaxis, :]**2))
            fTsumwy = np.sum(bins * y_centers[:, np.newaxis])
            fTsumwy2 = np.sum(bins * (y_centers[:, np.newaxis]**2))
            fTsumwxy = np.sum(bins * x_centers[np.newaxis, :] * y_centers[:, np.newaxis])

            file[name] = uproot.writing.identify.to_TH2x(
                fName=None,
                fTitle=title,
                data=data,
                fEntries=fTsumw,
                fTsumw=fTsumw,
                fTsumw2=fTsumw2,
                fTsumwx=fTsumwx,
                fTsumwx2=fTsumwx2,
                fTsumwy=fTsumwy,
                fTsumwy2=fTsumwy2,
                fTsumwxy=fTsumwxy,
                fSumw2=None,
                fXaxis=fXaxis,
                fYaxis=fYaxis
            )
            
            print(f"2D Histogram '{name}' written successfully.")
            
    print(f"All histograms written to '{output_file}'.")


if __name__ == "__main__":
    # Example 1D histograms
    hist1d_data = [
        ("test/hist1d_1", "Example 1D Histogram 1", [5, 10, 15], 1, 2, (0.0, 3.0)),
        ("test/hist1d_2", "Example 1D Histogram 2", [3, 6, 9, 12], 0, 1, (0.0, 4.0)),
    ]
    
    # Example 2D histograms with single underflow and overflow for X and Y
    hist2d_data = [
        (
            "hist2d_1", "Example 2D Histogram 1",
            [[1, 2, 3], [4, 5, 6], [7, 8, 9]],  # Main bin contents
            (0.0, 3.0), (0.0, 3.0)  # Range for X and Y
        ),
    ]


    # Output file
    output_file = "histograms.root"

    # Write histograms
    write_histograms(output_file, hist1d_data, hist2d_data)