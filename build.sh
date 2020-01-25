#!/bin/bash

set -xe

cargo clean --doc

cargo fmt --all -- --check
cargo test --release
cargo test --release --all-features
cargo bench --no-run

cargo doc --all-features
linkchecker target/doc/firmware/index.html
linkchecker target/doc/hal-comp/index.html
linkchecker target/doc/linuxcnc-hal-sys/index.html
