#!/usr/bin/env bash
set -x
set -eo pipefail

cargo fmt
cargo clippy -- -D warnings
[[ $? -eq 0 ]] && git commit -am "$1"
[[ $? -eq 0 ]] && git push
