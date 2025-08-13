cargo build --release
cargo build --release --target aarch64-unknown-linux-gnu
rm -f out/app_lev_orc_aarch64
mv target/aarch64-unknown-linux-gnu/release/app_lev_orc out/app_lev_orc_aarch64
mv target/release/app_lev_orc out/app_lev_orc_x64