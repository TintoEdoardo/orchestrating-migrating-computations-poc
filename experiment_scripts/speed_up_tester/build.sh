# Build for aarch64 and x86.
cargo build --release
cargo build --release --target aarch64-unknown-linux-gnu

# Remove previous build.
rm -f out/tester_aarch64
rm -f out/tester_x86

# Copy the new binaries in out.
cp target/aarch64-unknown-linux-gnu/release/speed_up_tester out/tester_aarch64
cp target/release/speed_up_tester out/tester_x86

# Copy a request.
cp ../../requests/0_0_req/module.wasm out/module.wasm