#!/bin/bash
# Build the project
cargo build --release

# Copy the binary to the mounted volume
cp target/release/kiosk_versioning /output/