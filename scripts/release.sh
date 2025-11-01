#!/bin/bash

set -e

echo "Building release binaries..."
cargo build --release --workspace

echo "Packaging binaries..."
# Add packaging commands here

echo "Signing binaries..."
# Add signing commands here

echo "Hashing artifacts..."
# Add hashing commands here
