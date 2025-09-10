# Build for aarch64 and x86.
cargo build --release
cargo build --release --target aarch64-unknown-linux-gnu

# Remove previous build.
rm -f out/app_lev_orc_aarch64
rm -f out/app_lev_orc_x86

# Remove existing `requests` folder.
rm -r out/requests &> /dev/null

# Copy the new binaries in out.
cp target/aarch64-unknown-linux-gnu/release/app_lev_orc out/app_lev_orc_aarch64
cp target/release/app_lev_orc out/app_lev_orc_x86

# Copy the original `requests` folder in `out/`.
cp -r requests out/requests