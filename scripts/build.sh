#!/bin/bash

# NovaSchool OS Developer Toolkit script
# This script builds, tests, or launches the NovaSchool OS emulator environment.

ACTION=${1:-run}

case "$ACTION" in
    "build")
        echo "Building NovaSchool OS Workspace..."
        cargo build --workspace
        ;;
    "run")
        echo "Booting NovaSchool OS..."
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
