#!/bin/bash -x
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

# ensure arg are valid for lk watch and lk filter
lk watch $* | lk filter $* | lk p "TX [hash:str]" &

ADDR=${ADDR:-0.0.0.0}
PORT=${PORT:-9090}

websocat --binary -E -e ws-listen:$ADDR:$PORT sh-c:"./io.sh $*"
