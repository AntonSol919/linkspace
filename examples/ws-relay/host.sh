#!/bin/bash -x
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

# ensure arg are valid for lk watch and lk filter
lk watch $* | lk filter $* | lk p "TX [hash:str]" &

export ACCEPTS=$(mktemp);
lk print-query $* --signed | lk keypoint handshake::/accepts --data-stdin --create-int 0 > $ACCEPTS

export ADDR=${ADDR:-0.0.0.0}
export PORT=${PORT:-9090}
export IO=${IO:-./chatroom.io.sh}

websocat --binary -E -e ws-listen:$ADDR:$PORT sh-c:"$IO $*"
