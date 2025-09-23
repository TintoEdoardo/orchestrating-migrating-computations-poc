# Build for aarch64 and x86.
cargo build
cargo build --target aarch64-unknown-linux-gnu

# Remove previous build.
rm -f ../lp_task_aarch64
rm -f ../lp_task_x86

# Copy the new binaries in out.
cp target/aarch64-unknown-linux-gnu/debug/low_priority_task ../lp_task_aarch64
cp target/debug/low_priority_task ../lp_task_x86
