
# This script demonstrates how to convert a Parquet file to a ROOT TTree using the `parquet_to_root` function
# from the `parquet_to_root` module.

# pip install parquet-to-root
# import ROOT must work for the script to run

from parquet_to_root import parquet_to_root
import time

start_time = time.time()

# Specify the input Parquet file and the output ROOT file
parquet_file = "./input_file.parquet"

root_file = "./output_file.root"
tree_name = "TreeName"  # Name of the ROOT TTree

# Convert the Parquet file to a ROOT TTree
parquet_to_root(parquet_file, root_file, treename=tree_name, verbose=True)

print(f"Conversion complete in {time.time() - start_time:.2f}s: {parquet_file} -> {root_file} with TTree name '{tree_name}'")