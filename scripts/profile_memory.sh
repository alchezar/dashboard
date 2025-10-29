#!/bin/bash

# --- Memory Profiling Script for the Server ---
# This script runs the main server application under Valgrind's Massif tool
# to profile heap memory usage.

# Build the server in release mode for accurate profiling.
echo "Building server in release mode..."
if ! (cd crates/server && cargo build --release); then
    echo "Build failed. Aborting."
    exit 1
fi

# Set environment variables required by the application's config loader.
APP_ENVIRONMENT=local
export APP_ENVIRONMENT
CARGO_MANIFEST_DIR="$(pwd)/crates/server"
export CARGO_MANIFEST_DIR

# Define an absolute path for the Massif report.
OUTPUT_FILE="$(pwd)/target/massif.out"

# Run the server with Valgrind/Massif.
echo ""
echo "Starting server with Valgrind/Massif..."
echo "The server will run VERY slowly. This is expected."
echo "Once the server is running, perform load testing in another terminal."
echo "Press Ctrl+C here when you are finished with load testing."
echo ""

valgrind --tool=massif --massif-out-file="$OUTPUT_FILE" ./target/release/dashboard_server

echo ""
echo "Profiling finished."
echo "To analyze the results, run: ms_print target/massif.out"
