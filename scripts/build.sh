#!/bin/bash

# NovaOS Developer Toolkit script
# This script builds, tests, or launches the NovaOS emulator environment.

ACTION=${1:-run}

case "$ACTION" in
    "build")
        echo "Building NovaOS Workspace..."
        cargo build --workspace
        ;;
    "run")
        echo "Booting NovaOS..."
        cargo run
        ;;
    "test")
        echo "Running kernel and VFS integration test suites..."
        cargo test --workspace
        ;;
    *)
        echo "Usage: ./build.sh [build|run|test]"
        exit 1
        ;;
esac
