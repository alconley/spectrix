#!/bin/bash

# Check if a virtual environment is already activated
if [[ -z "$VIRTUAL_ENV" ]]; then
    echo "No virtual environment detected."

    # Check if the .venv directory exists
    if [[ ! -d ".venv" ]]; then
        echo "Creating a new virtual environment..."
        python3 -m venv .venv
    else
        echo "Virtual environment already exists."
    fi

    # Activate the virtual environment
    echo "Activating the virtual environment..."
    source .venv/bin/activate
else
    echo "Virtual environment is already activated."
fi

# Install required Python packages
echo "Installing required Python packages..."
pip install -r requirements.txt

# Set the PYO3_PYTHON environment variable to point to the Python in the virtual environment
export PYO3_PYTHON=$(pwd)/.venv/bin/python
echo "PYO3_PYTHON set to $(pwd)/.venv/bin/python"

# Dynamically determine the Python version for PYTHONPATH
PYTHON_VERSION=$($PYO3_PYTHON -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
export PYTHONPATH=$(pwd)/.venv/lib/python${PYTHON_VERSION}/site-packages
echo "PYTHONPATH set to $(pwd)/.venv/lib/python${PYTHON_VERSION}/site-packages"

# Run the Rust project in release mode
echo "Running Rust project in release mode..."
cargo run --release