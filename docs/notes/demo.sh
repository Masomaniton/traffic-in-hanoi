cd crates/traffic-web
trunk build --release --dist ../traffic-server/static
cd ../..
cargo run -p traffic-server
