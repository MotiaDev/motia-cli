if [ "$1" == "linux" ]; then
  echo "Building for Linux"
  cargo build --release --target x86_64-unknown-linux-gnu
  mkdir -p .bin
  rm .bin/motia-cli || true
  mv target/x86_64-unknown-linux-gnu/release/motia-cli .bin/motia-cli
else
  echo "Building for current platform"
  cargo build --release
  mkdir -p .bin
  rm .bin/motia-cli || true
  mv target/release/motia-cli .bin/motia-cli
fi
