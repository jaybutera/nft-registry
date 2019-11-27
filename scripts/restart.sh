./scripts/build.sh
cargo build --release
./target/release/test purge-chain --dev
./target/release/test --dev
