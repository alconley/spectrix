import numpy as np
import uproot

def write_histograms(output_file, hist1d_data):
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
        for name, title, bins, underflow, overflow, range_ in hist1d_data:
            # Create bin edges for the histogram
            bin_edges = np.linspace(range_[0], range_[1], len(bins) + 1)
            
            # Include underflow and overflow in the data array
            data = np.array([underflow] + bins + [overflow], dtype=np.float32)
            bins_array = np.array(bins, dtype=np.float32)  # Convert bins to numpy array

            # Define fXaxis using to_TAxis with positional arguments
            fXaxis = uproot.writing.identify.to_TAxis(
                fName="xaxis",         # Temporary name for the X-axis
                fTitle="X-axis",       # Title of the X-axis
                fNbins=len(bins),      # Number of bins
                fXmin=range_[0],       # Minimum X-axis value
                fXmax=range_[1],       # Maximum X-axis value
                fXbins=bin_edges       # Bin edges
            )

            # Calculate metadata
            fEntries = float(np.sum(bins))
            fTsumw = float(np.sum(bins))
            fTsumw2 = float(np.sum(bins_array**2))
            fTsumwx = float(np.sum(bins_array * bin_edges[:-1]))
            fTsumwx2 = float(np.sum(bins_array * bin_edges[:-1]**2))
            fSumw2 = np.zeros(len(data), dtype=np.float64)

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

    print(f"All histograms written to '{output_file}'.")


if __name__ == "__main__":
    # Example 1D histograms
    hist1d_data = [
        ("hist1d_1", "Example 1D Histogram 1", [5, 10, 15], 1, 2, (0.0, 3.0)),
        ("hist1d_2", "Example 1D Histogram 2", [3, 6, 9, 12], 0, 1, (0.0, 4.0)),
    ]

    # Output file
    output_file = "test_histograms.root"

    # Write histograms
    write_histograms(output_file, hist1d_data)