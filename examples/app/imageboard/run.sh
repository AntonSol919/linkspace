#!/bin/bash -x
set -euo pipefail
IMG1=https://www.rust-lang.org/static/images/rust-logo-blk.svg;
curl -s $IMG1 | ./img-place.sh hello 20 20 -
IMG2=https://kernel.org/theme/images/logos/tux.png
curl -s $IMG2 | ./img-place.sh hello 220 120 -
./img-watch.sh hello 
