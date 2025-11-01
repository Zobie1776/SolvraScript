#!/bin/bash

set -e

echo "Running SolvraScript tests..."
cargo test -p solvrascript --tests

echo "Running SolvraCore tests with telemetry feature..."
cargo test -p solvracore --features telemetry
