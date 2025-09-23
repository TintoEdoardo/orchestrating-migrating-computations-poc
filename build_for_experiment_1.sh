#!/bin/bash

# Build for aarch64 and x86, in centralized mode.
cargo build --release --features=experiment_1_centralized
cargo build --release --target aarch64-unknown-linux-gnu --features=experiment_1_centralized

# Remove previous build.
rm -f out/app_lev_orc_aarch64_c
rm -f out/app_lev_orc_x86_c

# Copy the new binaries in out.
cp target/aarch64-unknown-linux-gnu/release/app_lev_orc out/app_lev_orc_aarch64_c
cp target/release/app_lev_orc out/app_lev_orc_x86_c

# Build for aarch64 and x86, in distributed mode.
cargo build --release --features=experiment_1_distributed
cargo build --release --target aarch64-unknown-linux-gnu --features=experiment_1_distributed

# Remove previous build.
rm -f out/app_lev_orc_aarch64_d
rm -f out/app_lev_orc_x86_d

# Copy the new binaries in out.
cp target/aarch64-unknown-linux-gnu/release/app_lev_orc out/app_lev_orc_aarch64_d
cp target/release/app_lev_orc out/app_lev_orc_x86_d

