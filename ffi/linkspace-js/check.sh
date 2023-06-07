#!/bin/bash
set -eu -o pipefail

wasm-objdump ./pkg/linkspace_bg.wasm -x
ls -lh ./pkg/linkspace_bg.wasm ./pkg/prev.wasm

has_name() {
    rg $1 <(strings ./pkg/linkspace_bg.wasm) || true
}

has_name "panic"
has_name "Permission"
has_name "aliasing"
has_name "unwrap"
