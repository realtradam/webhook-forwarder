#!/bin/bash
set -euo pipefail

cd /home/tradam/projects/webhook-forwarder

# Initialize Rust project (skips if Cargo.toml already exists)
if [ ! -f Cargo.toml ]; then
    cargo init --name webhook-forwarder
fi

# Add dependencies
cargo add hyper --features server,http1
cargo add hyper-util --features tokio
cargo add http-body-util
cargo add tokio --features rt-multi-thread,net,macros
cargo add bytes

echo "Done! Rust project scaffolded successfully."
