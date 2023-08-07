#!/usr/bin/env bash
set -x
set -eo pipefail

dnf install lld clang

cargo install cargo-watch
cargo install cargo-tarpaulin
cargo install cargo-audit
cargo install cargo-expand
