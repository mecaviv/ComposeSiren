bash build.dotnet.sh
cd ./Source/rust
cargo build
cd ../generated/rust/samples
cargo run --example sample