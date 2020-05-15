#!/bin/bash
python -m http.server &
RUST_LOG=info RUST_BACKTRACE=1 cargo run
