#!/bin/bash

# Build for aarch64 and x86, in centralized mode.
cargo build --release --no-default-features --features=experiment_2_centralized
cargo build --release --no-default-features --target aarch64-unknown-linux-gnu --features=experiment_2_centralized

# Remove previous build.
rm -f out/app_lev_orc_aarch64_c_2
rm -f out/app_lev_orc_x86_c_2

# Copy the new binaries in out.
cp target/aarch64-unknown-linux-gnu/release/app_lev_orc out/app_lev_orc_aarch64_c_2
cp target/release/app_lev_orc out/app_lev_orc_x86_c_2

# Build for aarch64 and x86, in no_live_mig mode.
cargo build --release --no-default-features --features=experiment_2_no_live_mig
cargo build --release --no-default-features --target aarch64-unknown-linux-gnu --features=experiment_2_no_live_mig

# Remove previous build.
rm -f out/app_lev_orc_aarch64_c_2_nlm
rm -f out/app_lev_orc_x86_c_2_nlm

# Copy the new binaries in out.
cp target/aarch64-unknown-linux-gnu/release/app_lev_orc out/app_lev_orc_aarch64_c_2_nlm
cp target/release/app_lev_orc out/app_lev_orc_x86_c_2_nlm

