#!/usr/bin/env bash

export RUSTFLAGS="${RUSTFLAGS:''}";

if [[ -z "$ALLOW_WARNINGS" ]]; then
    export RUSTFLAGS="$RUSTFLAGS -D warnings";
fi

if [[ "$TARGET" ]]; then
    rustup target add $TARGET;
else
    TARGET=$(rustup target list | grep '(default)' | cut -d ' ' -f1)
fi

cargo build --target="$TARGET"

if [[ -z "$SKIP_TESTS" ]]; then
    cargo test;
fi

if [[ "$DO_BENCHMARKS" ]]; then
    cargo test --benches;
fi